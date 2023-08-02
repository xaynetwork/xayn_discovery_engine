// Copyright 2023 Xayn AG
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as
// published by the Free Software Foundation, version 3.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.

use serde_json::{json, Error, Value};
use xayn_integration_tests::{test_app, TEST_EMBEDDING_SIZE, UNCHANGED_CONFIG};
use xayn_test_utils::assert_approx_eq;
use xayn_web_api::Ingestion;
use xayn_web_api_shared::{
    elastic::{BulkInstruction, SerdeDiscard},
    serde::json_object,
};

fn id(id: &str) -> Result<Value, Error> {
    serde_json::to_value(BulkInstruction::Index { id: &id })
}

fn emb(emb: &[f32]) -> Result<Value, Error> {
    Ok(json!({ "embedding": emb }))
}

// just to be sure that the behavior hasn't changed
#[test]
fn test_normalized_es_knn_scores() {
    test_app::<Ingestion, _>(UNCHANGED_CONFIG, |_, _, services| async move {
        let client = services
            .silo
            .elastic_client()
            .with_index(&services.tenant.tenant_id);
        const LEN: usize = TEST_EMBEDDING_SIZE / 2;
        let normalized = (LEN as f32).sqrt().recip();
        let embedding = [[normalized; LEN], [0.; LEN]].concat();

        let response = client
            .bulk_request::<SerdeDiscard>([
                id("d1"),
                emb(&embedding),
                id("d2"),
                emb(&[[0.; LEN], [normalized; LEN]].concat()),
                id("d3"),
                emb(&[[-normalized; LEN], [0.; LEN]].concat()),
            ])
            .await
            .unwrap();
        assert!(!response.errors);

        let scores = client
            .search_request::<String>(json_object!({
                "knn": {
                    "field": "embedding",
                    "query_vector": embedding,
                    "k": 5,
                    "num_candidates": 5,
                },
                "size": 5
            }))
            .await
            .unwrap();
        assert_eq!(scores.len(), 3);
        assert_approx_eq!(f32, scores["d1"], 1.);
        assert_approx_eq!(f32, scores["d2"], 0.5);
        assert_approx_eq!(f32, scores["d3"], 0., epsilon = 1e-7);

        Ok(())
    });
}
