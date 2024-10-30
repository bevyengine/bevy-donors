mod every_org;
mod stripe;

use crate::{every_org::get_every_org_donors, stripe::get_stripe_donors};
use chrono::{DateTime, TimeDelta, Utc};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    fs::File,
    io::{Read, Write},
};

static SPONSOR_THRESHOLD: i64 = 500;
static GRACE_PERIOD_IN_DAYS: i64 = 0;

#[tokio::main]
async fn main() {
    // read donor info
    let mut file = File::open("donor_info.toml").unwrap();
    let mut donor_info_str = String::new();
    file.read_to_string(&mut donor_info_str).unwrap();
    let donor_info: Donors = toml::from_str(&donor_info_str).unwrap();

    let mut donors = Vec::new();
    let now = Utc::now();

    // read stripe donors
    donors.extend(get_stripe_donors(now).await);

    // read every.org donors
    donors.extend(get_every_org_donors(now).await.unwrap());

    apply_donor_info(&mut donors, donor_info.donor);
    let metrics = compute_metrics(&donors);

    let donors_toml = toml::to_string_pretty(&Donors { donor: donors }).unwrap();
    let metrics_toml = toml::to_string_pretty(&metrics).unwrap();

    let mut file = File::create("donors.toml").unwrap();
    file.write_all(donors_toml.as_bytes()).unwrap();

    let mut file = File::create("metrics.toml").unwrap();
    file.write_all(metrics_toml.as_bytes()).unwrap();
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
                if let Some(true) = donor_info.anonymize {
                    donor.name = None;
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

pub(crate) fn is_past(now: DateTime<Utc>, payment_time: DateTime<Utc>) -> bool {
    now - payment_time > TimeDelta::try_days(31 + GRACE_PERIOD_IN_DAYS).unwrap()
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

#[derive(Debug, Serialize, Deserialize)]
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
    anonymize: Option<bool>,
}

#[derive(Default, Serialize)]
struct Metrics {
    monthly_dollars: i64,
    sponsors: usize,
    members: usize,
}
