use std::collections::HashMap;

use crate::{
    data::document::{Relevance, UserFeedback},
    DocumentHistory,
    DocumentId,
};

pub(crate) enum DocumentRelevance {
    Positive,
    Negative,
}

impl From<(Relevance, UserFeedback)> for DocumentRelevance {
    fn from(history: (Relevance, UserFeedback)) -> DocumentRelevance {
        match history {
            (Relevance::Low, UserFeedback::Irrelevant | UserFeedback::NotGiven) => {
                DocumentRelevance::Negative
            }
            _ => DocumentRelevance::Positive,
        }
    }
}

/// Classifies the documents into positive and negative documents based on the user feedback
/// and the relevance of the results.
pub(super) fn classify_documents_based_on_user_feedback<D>(
    matching_documents: Vec<(&DocumentHistory, D)>,
) -> (Vec<D>, Vec<D>) {
    let mut positive_docs = Vec::<D>::new();
    let mut negative_docs = Vec::<D>::new();

    for (history_doc, doc) in matching_documents.into_iter() {
        match (history_doc.relevance, history_doc.user_feedback).into() {
            DocumentRelevance::Positive => positive_docs.push(doc),
            DocumentRelevance::Negative => negative_docs.push(doc),
        }
    }

    (positive_docs, negative_docs)
}

#[cfg(test)]
pub(super) mod tests {
    use ndarray::{arr1, FixedInitializer};

    use super::*;
    use crate::{
        coi::{
            point::{tests::CoiPointConstructor, NegativeCoi, PositiveCoi},
            CoiId,
        },
        utils::to_vec_of_ref_of,
    };

    fn create_cois<FI: FixedInitializer<Elem = f32>, CP: CoiPointConstructor>(
        points: &[FI],
    ) -> Vec<CP> {
        if FI::len() == 0 {
            return Vec::new();
        }

        points
            .iter()
            .enumerate()
            .map(|(id, point)| CP::new(CoiId::mocked(id), arr1(point.as_init_slice())))
            .collect()
    }

    pub(crate) fn create_pos_cois(
        points: &[impl FixedInitializer<Elem = f32>],
    ) -> Vec<PositiveCoi> {
        create_cois(points)
    }

    pub(crate) fn create_neg_cois(
        points: &[impl FixedInitializer<Elem = f32>],
    ) -> Vec<NegativeCoi> {
        create_cois(points)
    }

    pub(crate) fn create_document_history(
        points: Vec<(Relevance, UserFeedback)>,
    ) -> Vec<DocumentHistory> {
        points
            .into_iter()
            .enumerate()
            .map(|(id, (relevance, user_feedback))| DocumentHistory {
                id: DocumentId::from_u128(id as u128),
                relevance,
                user_feedback,
                ..Default::default()
            })
            .collect()
    }

    #[test]
    fn test_user_feedback() {
        assert!(matches!(
            (Relevance::Low, UserFeedback::Irrelevant).into(),
            DocumentRelevance::Negative,
        ));

        assert!(matches!(
            (Relevance::Medium, UserFeedback::Irrelevant).into(),
            DocumentRelevance::Positive,
        ));

        assert!(matches!(
            (Relevance::High, UserFeedback::Irrelevant).into(),
            DocumentRelevance::Positive,
        ));

        assert!(matches!(
            (Relevance::High, UserFeedback::Relevant).into(),
            DocumentRelevance::Positive,
        ));

        assert!(matches!(
            (Relevance::Medium, UserFeedback::Relevant).into(),
            DocumentRelevance::Positive,
        ));

        assert!(matches!(
            (Relevance::Low, UserFeedback::Relevant).into(),
            DocumentRelevance::Positive,
        ));

        assert!(matches!(
            (Relevance::High, UserFeedback::NotGiven).into(),
            DocumentRelevance::Positive,
        ));

        assert!(matches!(
            (Relevance::Medium, UserFeedback::NotGiven).into(),
            DocumentRelevance::Positive,
        ));

        assert!(matches!(
            (Relevance::Low, UserFeedback::NotGiven).into(),
            DocumentRelevance::Negative,
        ));
    }
}
