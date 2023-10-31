![alt text](images/github-banner.png)

# Xayn Discovery Engine

Here you can find our Personalised Semantic Search and Recommendations service implementation.

To test our fully managed Personalised Semantic Search and Recommendations Plug-in, you can get started by requesting a test environment [here](https://www.xayn.com/developers#B2b-form) and having a look at our API documentation:

The Front Office API provides the API that is needed to get personalised content or do semantic searches. It is also used to transmit a user's interactions for personalisation. You can find its reference here: [Front Office API](https://xaynetwork.github.io/xayn_discovery_engine/front_office.html)

The back office allows you to define which pieces of your content shall be personalised for your users. You use the back office to add/remove your content pieces, which we refer to as “documents”. You can find its reference here: [Back Office API](https://xaynetwork.github.io/xayn_discovery_engine/back_office.html)

For more information, please visit our [website](https://www.xayn.com).

## Local Dependencies

When using `just` commands we will set environment variables appropriately
to install python and npm dependencies locally in the project.

Use `just install-tools` to install various tools (where possible they are installed
locally in the project tree).

When directly calling pipenv/npm/npx without going through `just` you might not have the right environment variables set.
For that case you can use `just run`, or either of `just pipenv`, `just npm` or `just npx`.

It can make sense to set `PIPENV_VENV_IN_PROJECT=1` in your user environment to make sure venvs are stored in the project tree and in turn deleted when you delete the project.

## License

See the [NOTICE](NOTICE) file.

### Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in this project by you, shall be licensed as AGPL-3.0-only, without any additional
terms or conditions.
