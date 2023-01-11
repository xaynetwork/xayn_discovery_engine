use xayn_ai_integration_test_utils::{web_dev_integration_test_setup, WebDevEnv};
use xayn_ai_test_utils::error::Panic;

/*

Problems:

1. how to pass arguments/config
2. some await nonsense
*/
#[ignore]
#[tokio::test]
async fn adding_documents_work() -> Result<(), Panic> {
    web_dev_integration_test_setup(
        |WebDevEnv {
             id: _,
             pg_uri: _,
             es_uri: _,
         }| {
            Box::pin(async {
                //spawn : run::<Ingestion>().await.unwrap();
                // also have an exit guard thingy
                //TODO
                Ok(())
            })
        },
    )
    .await
}
