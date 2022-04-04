use ndarray::arr1;

use crate::{
    coi::{compute_coi, config::Config, update_user_interests, RelevanceMap},
    data::document_data::{
        DocumentDataWithQAMBert,
        DocumentDataWithSMBert,
        QAMBertComponent,
        SMBertComponent,
    },
    tests::{MockCoiSystem, MockQAMBertSystem, MockSMBertSystem},
};

pub(crate) fn mocked_smbert_system() -> MockSMBertSystem {
    let mut mock_smbert = MockSMBertSystem::new();
    mock_smbert.expect_compute_embedding().returning(|docs| {
        Ok(docs
            .iter()
            .map(|doc| {
                let mut embedding: Vec<f32> = doc
                    .document_content
                    .clone()
                    .title
                    .into_bytes()
                    .into_iter()
                    .map(|c| c as f32)
                    .collect();
                embedding.resize(128, 0.);

                DocumentDataWithSMBert {
                    document_base: doc.document_base.clone(),
                    document_content: doc.document_content.clone(),
                    smbert: SMBertComponent {
                        embedding: arr1(&embedding).into(),
                    },
                }
            })
            .collect())
    });
    mock_smbert
}

pub(crate) fn mocked_qambert_system() -> MockQAMBertSystem {
    let mut mock_qambert = MockQAMBertSystem::new();
    mock_qambert.expect_compute_similarity().returning(|docs| {
        Ok(docs
            .iter()
            .map(|doc| {
                let snippet = &doc.document_content.snippet;
                let data = if snippet.is_empty() {
                    &doc.document_content.title
                } else {
                    snippet
                };
                let similarity =
                    (data.len() as f32 - doc.document_content.query_words.len() as f32).abs();

                DocumentDataWithQAMBert {
                    document_base: doc.document_base.clone(),
                    document_content: doc.document_content.clone(),
                    smbert: doc.smbert.clone(),
                    coi: doc.coi.clone(),
                    qambert: QAMBertComponent { similarity },
                }
            })
            .collect())
    });

    mock_qambert
}

fn mocked_coi_system() -> MockCoiSystem {
    let config = Config::default();

    let mut system = MockCoiSystem::new();
    system.expect_compute_coi().returning(compute_coi);
    system.expect_update_user_interests().returning(
        move |history, documents, user_interests, _| {
            update_user_interests(
                user_interests,
                &mut RelevanceMap::default(),
                history,
                documents,
                |_| todo!(/* mock once KPE is used */),
                &config,
            )
        },
    );
    system
}
