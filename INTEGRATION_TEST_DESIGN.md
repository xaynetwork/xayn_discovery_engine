
- for each test (there is no global setup/tear down)
    1. `podman ps --format json | jq -e 'map(select(.Labels."com.docker.compose.project" == "web-api") | .Labels."com.docker.compose.service") | contains(["elasticsearch","ingestion","personalization","postgres"])'` should not fail (`-e` sets the exit code based on the output of the pipe not being false/null and not error)
    2. if `false` fail
        - improvement: if failed start services, but also need some lock, and a process lock is probably not enough
    4. generate unique id
        - should contain a timestamp
        - as tests run in parallel a timestamp is not enough we also need some unique part
          - a process counter isn't enough either as there might be multiple processes
          - I guess a random id is the simplest part
    5. on postgres create db `test_<unique_id>`
        - includes waiting for PG to be ready
    6. on elastic search create index `test_<unique_id>` and setup schema
        - includes waiting for ES to be ready
    7. run test
        - clean env
            - or at least elasticsearch and postgress connection info
        - for this we need to export some parts form the library
        - if start ingestion/personalization then do so _in same process_
            - this avoids zombie services
            - but I feel there was some gotcha with processes connecting to them self
              but I can't remember
        - we might want to add a way to use actix integration and/or unit tests
        - do we expose the config or do we pass config by string?
            - at least we need to be able to access the db from tests directly and
              pass the ES/PG connection info to the test without using ENV overrides
              (as multiple tests run in the same process)
    8. cleanup
        - delete index with elastic search
        - delete db with postgres
        - should we run cleanup on failed tests?
            - probably not and cleanup all remainders is as simple as restarting PG & ES
              - so we probably should have a `just cleanup-tests`
- creating dbs and indices isn't supper expensive, still it's not cheap so we prefer viewer chained/longer tests
  over many very small ones


# Enable/Disable integration tests

Options:

0. just always run them and require `cargo test --lib` to not run them
1. using a feature gate, so `cargo test --all-features` also runs all tests
    - pro: pretty normal how it works
    - con: test options in features
2. using a non-feature flag like `cfg(integration_test)` (set with `rustflags = ["--cfg", "tokio_unstable"]`)
    - pro: allows multiple special non public feature flags
    - con: semi-hidden semi-stable not supper easy to use
3. `#[ignore]` integration tests and enable them per-hand
    - pro: not `feature` but still standard functionality
    - con: abuses `ignore`, **runs all `#[ignored]` parts including the mime benchmark, which will fail**
