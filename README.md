# Bevy Donors

The source of truth for current Bevy Donor data. This syncs data from Stripe, combines it with manual configuration in `donor_info.toml`, and generates donor.toml for use in the `bevy-website` donation pages.

## License

Except where noted (below and/or in individual files), all code in this repository is dual-licensed under either:

* MIT License ([LICENSE-MIT](LICENSE-MIT) or [http://opensource.org/licenses/MIT](http://opensource.org/licenses/MIT))
* Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or [http://www.apache.org/licenses/LICENSE-2.0](http://www.apache.org/licenses/LICENSE-2.0))

at your option.
This means you can select the license you prefer!

All logos in the `logos` folder, the contents of `donor_info.toml`, generated `donors.toml`, and generated `metrics.toml` are _not_ licensed for use outside of the Bevy project. 

This dual-licensing approach is the de-facto standard in the Rust ecosystem and there are [very good reasons](https://github.com/bevyengine/bevy/issues/2373) to include both.
