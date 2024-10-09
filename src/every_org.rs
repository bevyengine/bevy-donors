use crate::{is_past, Donor};
use chrono::{DateTime, Utc};
use reqwest::Client;
use serde::Deserialize;
use std::{error::Error, fs::File, num::ParseFloatError};
use thiserror::Error;

const DONOR_CSV_PATH: &str = "every_org_donors/donors.csv";
const BEVY_DONATIONS_BALANCE_ROUTE: &str = "https://api.www.every.org/api/nonprofits/958ff03c-9c7b-44a4-a66a-2fcc9b8dfed7/admin/donationsBalance";

async fn call_donation_balance_api() -> Result<DonationsBalance, reqwest::Error> {
    let client = Client::new();
    let auth_cookie =
        std::env::var("EVERY_ORG_SESSION_COOKIE").expect("Missing EVERY_ORG_SESSION_COOKIE in env");
    let response = client
        .get(BEVY_DONATIONS_BALANCE_ROUTE)
        .header("Cookie", auth_cookie)
        .send()
        .await?;
    response.json::<DonationsBalance>().await
}

pub(crate) async fn get_every_org_donors(now: DateTime<Utc>) -> Result<Vec<Donor>, Box<dyn Error>> {
    let mut csv_donors = Vec::<EveryOrgDonorCsv>::new();
    if std::fs::metadata(DONOR_CSV_PATH).is_ok() {
        let file = File::open(DONOR_CSV_PATH).unwrap();
        let mut reader = csv::Reader::from_reader(&file);
        for record in reader.deserialize() {
            csv_donors.push(record?);
        }
    } else {
        let donations_balance = call_donation_balance_api().await?;
        let csv = donations_balance.data.csv_data.rebuild_csv();
        let mut reader = csv::Reader::from_reader(csv.as_bytes());
        for record in reader.deserialize() {
            csv_donors.push(record?);
        }
    };
    let mut donors = Vec::new();
    for csv_donor in &csv_donors {
        let Ok(mut donor) = Donor::try_from(csv_donor) else {
            continue;
        };
        if csv_donor.recurring_donation_status.as_deref() == Some("Active") {
            donor.past = Some(false);
        } else {
            let last_donation = csv_donor.last_donation.as_deref().unwrap_or("");
            let past = if let Ok(payment_time) = DateTime::parse_from_str(last_donation, "%m/%d/%Y")
            {
                is_past(now, payment_time.to_utc())
            } else {
                true
            };

            donor.past = Some(past);
        }

        donors.push(donor);
    }

    Ok(donors)
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
pub(crate) struct EveryOrgDonorCsv {
    #[serde(rename = "Created")]
    created: Option<String>,
    #[serde(rename = "Charge id")]
    charge_id: Option<String>,
    #[serde(rename = "Partner donation id")]
    partner_donation_id: Option<String>,
    #[serde(rename = "Name")]
    name: Option<String>,
    #[serde(rename = "Donor id")]
    donor_id: Option<String>,
    #[serde(rename = "First name")]
    first_name: Option<String>,
    #[serde(rename = "Last name")]
    last_name: Option<String>,
    #[serde(rename = "Email")]
    email: Option<String>,
    #[serde(rename = "Profile Pic Cid")]
    profile_pic_cid: Option<String>,
    #[serde(rename = "Mailing List Opt-In")]
    mailing_list_opt_in: Option<String>,
    #[serde(rename = "Frequency")]
    frequency: Option<String>,
    #[serde(rename = "Amount")]
    amount: Option<String>,
    #[serde(rename = "Net amount")]
    net_amount: Option<String>,
    #[serde(rename = "3P Fee")]
    third_party_fee: Option<String>,
    #[serde(rename = "Slippage")]
    slippage: Option<String>,
    #[serde(rename = "Asset symbol")]
    asset_symbol: Option<String>,
    #[serde(rename = "Asset quantity")]
    asset_quantity: Option<String>,
    #[serde(rename = "Payment method")]
    payment_method: Option<String>,
    #[serde(rename = "Payment type")]
    payment_type: Option<String>,
    #[serde(rename = "Status")]
    status: Option<String>,
    #[serde(rename = "Disbursement method")]
    disbursement_method: Option<String>,
    #[serde(rename = "Sent")]
    sent: Option<String>,
    #[serde(rename = "Disbursement id")]
    disbursement_id: Option<String>,
    #[serde(rename = "Entry page")]
    entry_page: Option<String>,
    #[serde(rename = "Referrer")]
    referrer: Option<String>,
    #[serde(rename = "Referral partner")]
    referral_partner: Option<String>,
    #[serde(rename = "Fundraiser")]
    fundraiser: Option<String>,
    #[serde(rename = "Fundraiser creator")]
    fundraiser_creator: Option<String>,
    #[serde(rename = "Designation")]
    designation: Option<String>,
    #[serde(rename = "Public supporter")]
    public_supporter: Option<String>,
    #[serde(rename = "Public testimony")]
    public_testimony: Option<String>,
    #[serde(rename = "Private note")]
    private_note: Option<String>,
    #[serde(rename = "UTM Source")]
    utm_source: Option<String>,
    #[serde(rename = "UTM Medium")]
    utm_medium: Option<String>,
    #[serde(rename = "UTM Campaign")]
    utm_campaign: Option<String>,
    #[serde(rename = "DAF")]
    daf: Option<String>,
    #[serde(rename = "Refund Type")]
    refund_type: Option<String>,
    #[serde(rename = "Project id")]
    project_id: Option<String>,
    #[serde(rename = "Execution number")]
    execution_number: Option<String>,
    #[serde(rename = "Refunded charge id")]
    refunded_charge_id: Option<String>,
    #[serde(rename = "Recurring donation id")]
    recurring_donation_id: Option<String>,
    #[serde(rename = "Recurring donation status")]
    recurring_donation_status: Option<String>,
    // Cancelled fields
    #[serde(rename = "Last donation")]
    last_donation: Option<String>,
    #[serde(rename = "Donor")]
    donor: Option<String>,
    #[serde(rename = "Donated")]
    donated: Option<String>,
    #[serde(rename = "Frequency meta")]
    frequency_meta: Option<String>,
    #[serde(rename = "Donations")]
    donations: Option<String>,
    #[serde(rename = "Fundraised")]
    fundraised: Option<String>,
    #[serde(rename = "Fundraisers")]
    fundraisers: Option<String>,
    #[serde(rename = "Notes")]
    notes: Option<String>,
}

#[derive(Error, Debug)]
pub enum EveryOrgToDonorError {
    #[error("Every.org donor is a private supporter")]
    PrivateSupporter,
    #[error("No amount provided")]
    NoAmount,
    #[error("Failed to parse amount")]
    ParseAmountError(#[from] ParseFloatError),
}

impl TryFrom<&EveryOrgDonorCsv> for Donor {
    type Error = EveryOrgToDonorError;
    fn try_from(value: &EveryOrgDonorCsv) -> Result<Donor, EveryOrgToDonorError> {
        // This is _very_ important to check. We can't leak non-public information
        if value.public_supporter != Some("true".to_string()) {
            return Err(EveryOrgToDonorError::PrivateSupporter);
        }

        Ok(Donor {
            customer_id: match value.donor_id.as_deref() {
                Some("") | None => None,
                Some(v) => Some(format!("every.org:{v}")),
            },
            source: Some("every.org".to_string()),
            /* Public Fields */
            name: match value.name.as_deref() {
                Some("") | None => None,
                Some(v) => Some(v.to_string()),
            },
            amount: match value.amount.as_deref() {
                Some("") | None => return Err(EveryOrgToDonorError::NoAmount),
                Some(v) => {
                    let amount: f64 = v.parse()?;
                    Some(amount as i64)
                }
            },
            past: None,
            link: None,
            logo: None,
            logo_scale: None,
            square_logo: None,
            style: None,
        })
    }
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
struct DonationsBalance {
    message: String,
    data: Data,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
struct Data {
    #[serde(alias = "allTimeBalance")]
    all_time_balance: Amount,
    #[serde(alias = "allTimeRecurring")]
    all_time_recurring: Amount,
    #[serde(alias = "availableBalance")]
    available_balance: Amount,
    #[serde(alias = "currentBalance")]
    current_balance: Amount,
    #[serde(alias = "annualRecurringRevenue")]
    annual_recurring_revenue: Amount,
    #[serde(alias = "monthlyRecurringRevenue")]
    monthly_recurring_revenue: Amount,
    #[serde(alias = "giftCount")]
    gift_count: usize,
    #[serde(alias = "recurringSupporterCount")]
    recurring_supporter_count: usize,
    #[serde(alias = "usersFundraising")]
    users_fundraising: usize,
    #[serde(alias = "csvData")]
    csv_data: CsvData,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
struct Amount {
    amount: String,
    currency: String,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
struct CsvData {
    ok: Vec<Vec<String>>,
    error: Vec<Vec<String>>,
}

impl CsvData {
    fn rebuild_csv(&self) -> String {
        let mut csv = String::new();
        for row in &self.ok {
            for column in row {
                // disallow commas as this breaks the CSV
                let column = column.replace(',', " ");
                csv.push_str(&column);
                csv.push(',');
            }
            csv.push('\n');
        }

        csv
    }
}
