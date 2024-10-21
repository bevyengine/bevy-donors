# Bevy Donors

The source of truth for current Bevy Donor data. This syncs data from Every.org and Stripe, combines it with manual configuration in `donor_info.toml`, and generates donor.toml for use in the `bevy-website` donation pages.

## How This Works

This repo periodically runs the `update_donors.yml` GitHub workflow, which pulls current donor info from Stripe, merges it with `donor_info.toml`, and generates two files:

1. `donors.toml`: a formatted list of all current and past donors and their relevant metadata (Stripe customer id, amount donated, name, link, logo, etc)
2. `metrics.toml`: metrics computed from the data in `donors.toml` (total monthly donations in USD, donor count, sponsor count)

Entries in `donor_info.toml` that specify a Stripe or Every.org `customer_id` field will be merged with the relevant Stripe data. If a field exists both in the Stripe or Every.org data and the `donor_info.toml`, the `donor_info.toml` value will override the Stripe or Every.org value. For example, you can manually override a link if the donor wants to change it to something else. Entries that _do not_ specify a `customer_id` will be treated as "manual" entries. Manual entries are generally for donors that donate without Stripe or Every.org via bank transfers.

All metrics are automatically updated based on current Every.org and Stripe donor information.

## Adding/Updating Donor Info

First: if you are a donor reading this, note that we are happy to do all of this for you. Feel free to open an issue here or reach out to [bevyengine@gmail.com](mailto:bevyengine@gmail.com) with whatever you want. You are also welcome to make your own changes via a pull request changing/adding your entry to `donor_info.toml`.

For non-logo tiers, if a donor entered their name or link in the Stripe form, the donor will automatically be added in the next workflow run (at the time of writing, this happens every 8 hours).

If an Every.org or Stripe donor needs to add/change information (name, logo, link, etc), then an entry should be added to `donor_info.toml` with donor's `customer_id`. Any fields set in this entry will override whatever has been set via the Every.org or Stripe data. Take a look at the [existing donor_info.toml entries](donor_info.toml) for examples.

### Finding the `customer_id`

For customers that already filled in some information on Every.org or Stripe when they started their donation, the easiest way to find the `customer_id` is to download the latest [`donors.toml` release](https://github.com/bevyengine/bevy-donors/releases) (make sure one has happened since the donation happened) and search for that identifying information. The `customer_id` will be in that entry.

Every.org donors can also log into [every.org](https://every.org) and _then_ navigate to <https://api.www.every.org/api/me>. This will return a JSON response. `data.user.id` is your every.org donor id. Your `customer_id` is just `every.org:DONOR_ID`.

If it cannot be found that way, create an issue here or reach out to [support@bevyengine.org](mailto:support@bevyengine.org) with your request. We can log into Every.org or Stripe and use other transaction information to find your `customer_id`. Generally the name and tier are enough to correlate to the `customer_id`.

### Logo tiers

Logo tiers need to add their logo to the `logos` folder and add an entry to `donor_info.toml` with `logo = "LOGO.EXTENSION"` (ex: `logo = "my_logo.png"`).

Logos are assumed to have a "width dominant" aspect ratio. If a logo is square / roughly square, use `square_logo = true` to give it a slight scale boost (in the interest of visibility fairness with non-square logos). On a case-by-case basis (in the interest of "visibility fairness"), the scale can be fully overridden using `logo_scale = FLOAT_VALUE`.

## Authentication

1. For Stripe integration, generate a new API key and set the STRIPE_SECRET_KEY environment variable (for Github Actions, set this as a bevyengine org secret).
2. For Every.org integration, log in to Every.org with an admin account, grab the session stored in `Cookie` and then set it as the EVERY_ORG_SESSION_COOKIE environment variable (for Github Actions, set this as a bevyengine org secret).

## License

Except where noted (below and/or in individual files), all code in this repository is dual-licensed under either:

* MIT License ([LICENSE-MIT](LICENSE-MIT) or [http://opensource.org/licenses/MIT](http://opensource.org/licenses/MIT))
* Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or [http://www.apache.org/licenses/LICENSE-2.0](http://www.apache.org/licenses/LICENSE-2.0))

at your option.
This means you can select the license you prefer!

All logos in the `logos` folder, the contents of `donor_info.toml`, generated `donors.toml`, and generated `metrics.toml` are _not_ licensed for use outside of the Bevy project. 

This dual-licensing approach is the de-facto standard in the Rust ecosystem and there are [very good reasons](https://github.com/bevyengine/bevy/issues/2373) to include both.


hi this is updated readme