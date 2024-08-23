use chrono::{DateTime, TimeDelta, Utc};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    fs::File,
    io::{Read, Write},
};
use stripe::{
    CheckoutSession, Client, Currency, ListCheckoutSessions, ListPaymentIntents, PaymentIntent,
    PaymentIntentStatus,
};

static SPONSOR_THRESHOLD: i64 = 500;
static GRACE_PERIOD_IN_DAYS: i64 = 0;

#[tokio::main]
async fn main() {
    let secret_key = std::env::var("STRIPE_SECRET_KEY").expect("Missing STRIPE_SECRET_KEY in env");
    let client = Client::new(secret_key);

    let mut file = File::open("donor_info.toml").unwrap();
    let mut donor_info_str = String::new();
    file.read_to_string(&mut donor_info_str).unwrap();

    let donor_info: Donors = toml::from_str(&donor_info_str).unwrap();

    let payment_intents = get_payment_intents(&client).await;
    let checkout_sessions = get_checkout_sessions(&client).await;

    let mut donor = compute_stripe_donors(&payment_intents, &checkout_sessions);
    apply_donor_info(&mut donor, donor_info.donor);
    let metrics = compute_metrics(&donor);

    let donors_toml = toml::to_string_pretty(&Donors { donor }).unwrap();
    let metrics_toml = toml::to_string_pretty(&metrics).unwrap();

    let mut file = File::create("donors.toml").unwrap();
    file.write_all(donors_toml.as_bytes()).unwrap();

    let mut file = File::create("metrics.toml").unwrap();
    file.write_all(metrics_toml.as_bytes()).unwrap();
}

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

/// Apply info from `donor_info.toml` to the donors list
/// This will merge into donors that already exist in the list (by writing on top) and create
/// donors that do not yet exist.
fn apply_donor_info(donors: &mut Vec<Donor>, donor_info: Vec<Donor>) {
    let mut customer_id_to_donor_info = HashMap::new();
    let mut donor_info_without_customer_ids = Vec::new();
    for donor_info in donor_info {
        if let Some(id) = &donor_info.customer_id {
            customer_id_to_donor_info.insert(id.clone(), donor_info);
        } else {
            donor_info_without_customer_ids.push(donor_info);
        }
    }

    for donor in donors.iter_mut() {
        if let Some(id) = &donor.customer_id {
            if let Some(donor_info) = customer_id_to_donor_info.remove(id) {
                if let Some(name) = &donor_info.name {
                    donor.name = Some(name.clone());
                }

                if let Some(link) = &donor_info.link {
                    donor.link = Some(link.clone());
                }

                if let Some(logo) = &donor_info.logo {
                    donor.logo = Some(logo.clone());
                }

                if let Some(style) = &donor_info.style {
                    donor.style = Some(style.clone());
                }

                if let Some(amount) = &donor_info.amount {
                    donor.amount = Some(*amount);
                }

                if let Some(square_logo) = &donor_info.square_logo {
                    donor.square_logo = Some(*square_logo);
                }

                if let Some(scale) = &donor_info.logo_scale {
                    donor.logo_scale = Some(*scale);
                }
            }
        }
    }

    for donor in donor_info_without_customer_ids {
        // only add donors that have an amount listed
        if donor.amount.is_some() {
            donors.push(donor);
        }
    }
}

fn compute_stripe_donors(
    payment_intents: &[PaymentIntent],
    checkout_sessions: &[CheckoutSession],
) -> Vec<Donor> {
    let mut donors = HashMap::new();

    let mut checkouts = HashMap::new();
    let now = Utc::now();
    for checkout_session in checkout_sessions {
        if checkout_session.status != Some(stripe::CheckoutSessionStatus::Complete) {
            continue;
        }

        let payment_intent = checkout_session.payment_intent.as_ref().expect("Payment intent was not included. Code currently assumes all CheckoutSessions have payment intents.");
        let payment_intent_id = payment_intent.id();

        checkouts
            .entry(payment_intent_id)
            .or_insert_with(Vec::new)
            .push(checkout_session.clone());
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

        let checkout = checkouts
            .get(&payment_intent.id)
            .expect(
                "Successful payment intents are expected to have at least one checkout session.",
            )
            .into_iter()
            .max_by_key(|c| c.created)
            .expect(
                "Successful payment intent should have a matching checkout with the same amount.",
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

        let past = now - payment_time > TimeDelta::try_days(31 + GRACE_PERIOD_IN_DAYS).unwrap();

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

fn compute_metrics(donors: &[Donor]) -> Metrics {
    let mut metrics = Metrics::default();
    for donor in donors {
        if donor.past.unwrap_or(false) {
            continue;
        }

        if let Some(amount) = donor.amount {
            metrics.monthly_dollars += amount;
            if amount >= SPONSOR_THRESHOLD {
                metrics.sponsors += 1;
            } else {
                metrics.members += 1;
            }
        }
    }

    metrics
}

#[derive(Serialize, Deserialize)]
struct Donors {
    donor: Vec<Donor>,
}

#[derive(Serialize, Deserialize)]
struct Donor {
    customer_id: Option<String>,
    name: Option<String>,
    link: Option<String>,
    logo: Option<String>,
    amount: Option<i64>,
    source: Option<String>,
    style: Option<String>,
    past: Option<bool>,
    square_logo: Option<bool>,
    logo_scale: Option<f32>,
}

#[derive(Default, Serialize)]
struct Metrics {
    monthly_dollars: i64,
    sponsors: usize,
    members: usize,
}
