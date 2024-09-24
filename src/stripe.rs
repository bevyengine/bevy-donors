use crate::{is_past, Donor};
use chrono::{DateTime, Utc};
use std::collections::HashMap;
use stripe::{
    CheckoutSession, Client, Currency, ListCheckoutSessions, ListPaymentIntents, PaymentIntent,
    PaymentIntentStatus,
};

async fn get_payment_intents(client: &Client) -> Vec<PaymentIntent> {
    let params = ListPaymentIntents {
        ..Default::default()
    };

    let mut paginate = PaymentIntent::list(client, &params)
        .await
        .unwrap()
        .paginate(params);

    let mut objects = Vec::new();
    loop {
        for object in &paginate.page.data {
            objects.push(object.clone());
        }

        if paginate.page.has_more {
            paginate = paginate.next(&client).await.unwrap();
        } else {
            break;
        }
    }

    objects
}

async fn get_checkout_sessions(client: &Client) -> Vec<CheckoutSession> {
    let params = ListCheckoutSessions {
        ..Default::default()
    };

    let mut paginate = CheckoutSession::list(client, &params)
        .await
        .unwrap()
        .paginate(params);

    let mut objects = Vec::new();
    loop {
        for object in &paginate.page.data {
            objects.push(object.clone());
        }

        if paginate.page.has_more {
            paginate = paginate.next(&client).await.unwrap();
        } else {
            break;
        }
    }

    objects
}

pub(crate) async fn get_stripe_donors(now: DateTime<Utc>) -> Vec<Donor> {
    let secret_key = std::env::var("STRIPE_SECRET_KEY").expect("Missing STRIPE_SECRET_KEY in env");
    let client = Client::new(secret_key);

    let payment_intents = get_payment_intents(&client).await;
    let checkout_sessions = get_checkout_sessions(&client).await;

    let mut donors = HashMap::new();

    let mut customer_id_to_checkouts = HashMap::new();
    for checkout_session in checkout_sessions {
        if checkout_session.status != Some(stripe::CheckoutSessionStatus::Complete) {
            continue;
        }

        let customer = checkout_session.customer.as_ref().expect("Customer information was not included. Code currently assumes all complete checkouts have customers.");
        let customer_id = customer.id();

        let checkouts = customer_id_to_checkouts
            .entry(customer_id)
            .or_insert_with(Vec::new);

        checkouts.push(checkout_session.clone());
    }

    // Ensure checkouts are sorted by the time they were created
    for checkouts in customer_id_to_checkouts.values_mut() {
        checkouts.sort_by_key(|c| c.created);
    }

    let mut payment_intents = payment_intents.to_vec();
    payment_intents.sort_by_key(|pi| pi.created);
    for payment_intent in payment_intents {
        if payment_intent.status != PaymentIntentStatus::Succeeded {
            continue;
        }

        if payment_intent.currency != Currency::USD {
            panic!("Encountered non-USD currency. Automation does not know how to handle currency conversion.");
        }

        let customer = payment_intent.customer.as_ref().expect("Customer information was not included. Code currently assumes all succeeded PaymentIntents have customers.");
        let customer_id = customer.id();

        let checkouts = customer_id_to_checkouts.get(&customer_id).expect(
            "Successful payment intents are expected to have at least one checkout session",
        );

        let checkout = checkouts
            .iter()
            .rev()
            .find(|c| c.amount_total == Some(payment_intent.amount))
            .expect(
                "Successful payment intent should have a matching checkout with the same amount",
            );

        let mut link = None;
        let mut name = None;

        for field in &checkout.custom_fields {
            let value = field.text.as_ref().and_then(|t| t.value.clone());
            match field.key.as_str() {
                "nametolistinbevycredits" => name = value,
                "linktolistinbevycredits" => link = value,
                _ => {}
            }
        }

        let payment_time = DateTime::from_timestamp(payment_intent.created, 0).unwrap();
        let past = is_past(now, payment_time);
        donors.insert(
            customer_id.to_string(),
            Donor {
                customer_id: Some(customer_id.to_string()),
                // convert from cents to dollars
                amount: Some(payment_intent.amount / 100),
                link,
                name,
                source: Some("stripe".to_string()),
                past: Some(past),
                logo: None,
                style: None,
                square_logo: None,
                logo_scale: None,
            },
        );
    }

    donors.into_values().collect()
}
