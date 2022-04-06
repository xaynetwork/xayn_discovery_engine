// Copyright 2021 Xayn AG
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

use crate::data::document::{Relevance, UserFeedback};

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

#[cfg(test)]
pub(super) mod tests {
    use ndarray::{arr1, FixedInitializer};

    use super::*;
    use crate::coi::{
        point::{tests::CoiPointConstructor, NegativeCoi, PositiveCoi},
        CoiId,
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
