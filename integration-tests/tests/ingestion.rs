use integration_tests::{send_assert_success, test_app};
use serde_json::json;
use xayn_web_api::Ingestion;

#[tokio::test]
async fn test_test_app() {
    test_app::<Ingestion, _>(
        |_config| {},
        |client, url, _service| async move {
            send_assert_success(
                &client,
                client
                    .post(url.join("/documents")?)
                    .json(&json!({
                        "documents": [
                            { "id": "d1", "snippet": "once in a spring there was a fall" },
                            { "id": "d2", "snippet": "fall in a once" },
                        ]
                    }))
                    .build()?,
            )
            .await;
            Ok(())
        },
    )
    .await;
}
