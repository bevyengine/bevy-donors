use crate::{is_past, Donor};
use chrono::{DateTime, Utc};
use serde::Deserialize;
use std::{error::Error, fs::File};

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
pub(crate) struct EveryOrgDonor {
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

pub(crate) fn get_every_org_donors(now: DateTime<Utc>) -> Result<Vec<Donor>, Box<dyn Error>> {
    let file = File::open("every_org_donors/donors.csv").unwrap();
    let mut reader = csv::Reader::from_reader(&file);
    let mut donors = Vec::new();
    for record in reader.deserialize() {
        let every_org_donor: EveryOrgDonor = record?;
        let Ok(mut donor) = Donor::try_from(&every_org_donor) else {
            continue;
        };
        if every_org_donor.recurring_donation_status.as_deref() == Some("Active") {
            donor.past = Some(false);
        } else {
            let last_donation = every_org_donor.last_donation.as_deref().unwrap_or("");
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

impl TryFrom<&EveryOrgDonor> for Donor {
    type Error = anyhow::Error;
    fn try_from(value: &EveryOrgDonor) -> Result<Donor, anyhow::Error> {
        // This is _very_ important to check. We can't leak non-public information
        let public_supporter = value.public_supporter == Some("true".to_string());
        Ok(Donor {
            customer_id: match value.donor_id.as_deref() {
                Some("") | None => None,
                Some(v) => Some(v.to_string()),
            },
            source: Some("every.org".to_string()),
            /* Public Fields */
            name: match (public_supporter, value.name.as_deref()) {
                (false, _) | (true, Some("")) | (true, None) => None,
                (true, Some(v)) => Some(v.to_string()),
            },
            amount: match (public_supporter, value.amount.as_deref()) {
                (false, _) | (true, Some("")) | (true, None) => None,
                (true, Some(v)) => {
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
