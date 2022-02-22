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

use crate::expression::Expr;

/// Filter the data using custom criteria.
#[derive(Default, Clone, Debug)]
pub struct Filter {
    keywords: Vec<String>,
}

impl Filter {
    /// Add a keyword to filter with. All keyword are in "or" with each other.
    pub fn add_keyword(mut self, keyword: impl Into<String>) -> Self {
        self.keywords.push(keyword.into());

        self
    }

    /// Build the expression.
    pub(crate) fn build(&self) -> String {
        let keywords = Expr::or_from_iter(self.keywords.iter().map(|k| format!("\"{}\"", k)));
        keywords.build()
    }
}

/// Define area and language of interests.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Market {
    /// Country code as defined in ISO 3166-1 alpha-2.
    pub country_code: String,
    /// Language code as defined in ISO 639-1 — 2 letter code, e.g. 'de' or 'en'
    pub lang_code: String,
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
        let expected = "\"a b\" OR \"c d\"";
        let filter = Filter::default().add_keyword("a b").add_keyword("c d");
        assert_eq!(expected, filter.build());

        let filter = Filter::default().add_keyword("c d").add_keyword("a b");
        assert_eq!(expected, filter.build());
    }
}
