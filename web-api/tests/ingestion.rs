// use xayn_integration_test_utils::{web_dev_integration_test_setup, Services};
// use xayn_ai_test_utils::error::Panic;

// #[tokio::test]
// async fn test_adding_documents_work() -> Result<(), Panic> {
//     web_dev_integration_test_setup(
//         |Services {
//              id: _,
//              postgres: _,
//              elastic_search: _,
//          }| {
//             Box::pin(async {
//                 // format!() to create a toml config with pg_uri and then
//                 // pass it as `inline:` config to run of Ingestion/Personalization
//                 // spawn : run::<Ingestion>().await.unwrap();
//                 // also have an exit guard thingy
//                 //TODO
//                 Ok(())
//             })
//         },
//     )
//     .await
// }
