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

use displaydoc::Display;
use thiserror::Error;

use crate::{expression::Expr, languages};

/// Filter the data using custom criteria.
#[derive(Default)]
pub struct Filter {
    keywords: Vec<String>,
    markets: Vec<Market>,
    site_types: Vec<SiteType>,
}

impl Filter {
    /// Add a keyword to filter with. All keyword are in "or" with each other.
    pub fn add_keyword(mut self, keyword: impl Into<String>) -> Self {
        self.keywords.push(keyword.into());

        self
    }

    /// Add a market to filter with. All market are in "or" with each other.
    pub fn add_market(mut self, market: Market) -> Self {
        self.markets.push(market);

        self
    }

    /// Add a [`SiteType`] to filter with. All site types are in "or" with each other.
    pub fn add_site_type(mut self, site_type: SiteType) -> Self {
        self.site_types.push(site_type);

        self
    }

    /// Build the expression.
    pub(crate) fn build(&self) -> String {
        let keywords = Expr::or_from_iter(self.keywords.iter().map(|k| format!("\"{}\"", k)));
        let markets = Expr::or_from_iter(self.markets.iter());
        let site_types = Expr::or_from_iter(self.site_types.iter());

        keywords.and(markets).and(site_types).build()
    }
}

/// Define area and language of interests.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Market {
    /// Country code as defined in ISO 3166-1 alpha-2.
    country: String,
    /// Name of the name in English.
    language: String,
}

impl Market {
    /// Craate a new `Market`.
    ///
    /// `country` must be as defined in ISO 3166-1 alpha-2.
    /// `language` must be as in ISO ISO 639-1 definition.
    pub fn new(country: impl Into<String>, language: &str) -> Result<Self, Error> {
        let language = languages::get_name(language).ok_or(Error::InvalidLanguageCode)?;
        let country = country.into();
        Ok(Self { country, language })
    }
}

impl From<&Market> for Expr {
    fn from(market: &Market) -> Self {
        let country: Expr = format!("thread.country:{}", market.country).into();
        country.and(format!("language:{}", market.language))
    }
}

/// Type of website.
#[derive(PartialEq, Eq, PartialOrd, Ord)]
pub enum SiteType {
    /// News website.
    News,
    /// Blogs website.
    Blogs,
    /// Discussions website.
    Discussions,
}

impl From<&SiteType> for Expr {
    fn from(site_type: &SiteType) -> Self {
        format!(
            "site_type:{}",
            match site_type {
                SiteType::News => "news",
                SiteType::Blogs => "blogs",
                SiteType::Discussions => "discussions",
            }
        )
        .into()
    }
}

#[derive(Error, Display, Debug)]
pub enum Error {
    /// Language is not a valid code or is not supported.
    InvalidLanguageCode,
}

#[cfg(test)]
mod tests {
    use claim::assert_matches;

    use super::*;

    #[test]
    fn test_markets() {
        let market = Market::new("en", "en").unwrap();

        assert_eq!("en", market.country);
        assert_eq!("english", market.language);
    }

    #[test]
    fn test_markets_invalid_language() {
        assert_matches!(
            Market::new("en", "invalid").unwrap_err(),
            Error::InvalidLanguageCode
        );
    }

    #[test]
    fn test_filter_empty() {
        assert_eq!("", Filter::default().build());
    }

    #[test]
    fn test_filter_keywords() {
        let expected = "(\"a b\" OR \"c d\")";
        let filter = Filter::default().add_keyword("a b").add_keyword("c d");
        assert_eq!(expected, filter.build());

        let filter = Filter::default().add_keyword("c d").add_keyword("a b");
        assert_eq!(expected, filter.build());
    }

    #[test]
    fn test_filter_markets() {
        let en_market = Market::new("en", "en").unwrap();
        let en_market_str = "(language:english AND thread.country:en)";
        let de_market = Market::new("de", "de").unwrap();
        let de_market_str = "(language:german AND thread.country:de)";
        let expected = format!("({} OR {})", en_market_str, de_market_str);

        let filter = Filter::default()
            .add_market(en_market.clone())
            .add_market(de_market.clone());
        assert_eq!(expected, filter.build());

        let filter = Filter::default()
            .add_market(de_market)
            .add_market(en_market);
        assert_eq!(expected, filter.build());
    }

    #[test]
    fn test_filter_site_types() {
        let expected = "(site_type:blogs OR site_type:news)";

        let filter = Filter::default()
            .add_site_type(SiteType::Blogs)
            .add_site_type(SiteType::News);
        assert_eq!(expected, filter.build());

        let filter = Filter::default()
            .add_site_type(SiteType::News)
            .add_site_type(SiteType::Blogs);
        assert_eq!(expected, filter.build());
    }

    #[test]
    fn test_filter_all() {
        let en_market = Market::new("en", "en").unwrap();
        let en_market_str = "(language:english AND thread.country:en)";
        let de_market = Market::new("de", "de").unwrap();
        let de_market_str = "(language:german AND thread.country:de)";
        let markets_expected = format!("({} OR {})", en_market_str, de_market_str);
        let keywords_expected = "(\"a b\" OR \"c d\")";
        let site_types_expected = "(site_type:blogs OR site_type:news)";
        let expected = format!(
            "{} AND {} AND {}",
            markets_expected, keywords_expected, site_types_expected
        );

        let filter = Filter::default()
            .add_keyword("a b")
            .add_keyword("c d")
            .add_market(en_market)
            .add_market(de_market)
            .add_site_type(SiteType::Blogs)
            .add_site_type(SiteType::News);

        assert_eq!(expected, filter.build());
    }
}
