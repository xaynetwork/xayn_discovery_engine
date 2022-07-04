// Copyright 2022 Xayn AG
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

//! Filter the data from the provider.

use serde::{Deserialize, Serialize};

use crate::expression::Expr;

/// Filter the data using custom criteria.
#[derive(Default, Clone, Debug)]
pub struct Filter {
    keywords: Vec<String>,
}

impl Filter {
    /// Add a keyword to filter with. All keyword are in "or" with each other.
    ///
    /// Words in a key phase must not match `OR` or `AND`
    /// as they would interfere with the OR/AND query operators.
    #[must_use = "dropped changed filter"]
    pub fn add_keyword(mut self, keyword: &str) -> Self {
        // `"` can interfere with the exact match operator
        self.keywords.push(keyword.replace('"', ""));

        self
    }

    /// Build the expression.
    pub(crate) fn build(&self) -> String {
        if self.keywords.is_empty() {
            "*".into()
        } else {
            let keywords = Expr::or_from_iter(self.keywords.iter().map(|k| format!("({})", k)));
            keywords.build()
        }
    }
}

/// Define area and language of interests.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, Hash)]
pub struct Market {
    /// Country code as defined in ISO 3166-1 alpha-2.
    pub country_code: String,
    /// Language code as defined in ISO 639-1 — 2 letter code, e.g. 'de' or 'en'
    pub lang_code: String,
}

impl Market {
    /// Returns the default quality rank limit for news.
    pub fn news_quality_rank_limit(&self) -> Option<usize> {
        #[allow(clippy::match_same_arms)]
        Some(match &*self.country_code {
            "AT" => 70_000,
            "BE" => 70_000,
            "CA" => 70_000,
            "CH" => 50_000,
            "DE" => 9_000,
            "ES" => 40_000,
            "GB" => 14_000,
            "IE" => 70_000,
            "NL" => 60_000,
            "PL" => 50_000,
            "US" => 9_000,
            _ => return None,
        })
    }

    /// Returns the default quality rank limit for latest headlines.
    pub fn headlines_quality_rank_limit(&self) -> Option<usize> {
        #[allow(clippy::match_same_arms)]
        Some(match &*self.country_code {
            "AT" => 70_000,
            "BE" => 70_000,
            "CA" => 70_000,
            "CH" => 50_000,
            "DE" => 9_000,
            "ES" => 40_000,
            "GB" => 14_000,
            "IE" => 70_000,
            "NL" => 60_000,
            "PL" => 50_000,
            "US" => 9_000,
            _ => return None,
        })
    }
}

impl<C, L> From<(C, L)> for Market
where
    C: Into<String>,
    L: Into<String>,
{
    fn from((country_code, lang_code): (C, L)) -> Self {
        Self {
            country_code: country_code.into(),
            lang_code: lang_code.into(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_filter_empty() {
        assert_eq!("", Filter::default().build());
    }

    #[test]
    fn test_filter_keywords() {
        let expected = "(a b) OR (c d)";
        let filter = Filter::default().add_keyword("a b").add_keyword("c d");
        assert_eq!(expected, filter.build());

        let filter = Filter::default().add_keyword("c d").add_keyword("a b");
        assert_eq!(expected, filter.build());
    }

    #[test]
    fn test_filter_remove_invalid_char() {
        let filter = Filter::default().add_keyword("a\"b");
        assert_eq!("(ab)", filter.build());
    }

    #[test]
    fn test_empty_filter_is_wildcard() {
        assert_eq!(Filter::default().build(), "*");
    }
}
