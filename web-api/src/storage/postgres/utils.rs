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

use std::{fmt::Display, str::FromStr};

use once_cell::sync::Lazy;
use regex::Regex;
use thiserror::Error;

/// A quoted postgres identifier.
///
/// This can be used for cases where a SQL query is build
/// dynamically and is parameterized over an identifier in
/// a position where postgres doesn't allow `$` bindings.
///
/// For example in `SET ROLE "role";`
///
/// Be aware that quoted identifiers are case-sensitive and limited to 63 bytes.
/// Moreover, we only allow printable us-ascii characters excluding `"`; this is stricter than [postgres](https://www.postgresql.org/docs/15/sql-syntax-lexical.html#SQL-SYNTAX-IDENTIFIERS).
#[derive(Debug, Clone)]
pub(crate) struct QuotedIdentifier(String);

impl FromStr for QuotedIdentifier {
    type Err = InvalidQuotedIdentifier;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        s.to_owned().try_into()
    }
}

impl TryFrom<String> for QuotedIdentifier {
    type Error = InvalidQuotedIdentifier;

    fn try_from(identifier: String) -> Result<Self, Self::Error> {
        static RE: Lazy<Regex> = Lazy::new(|| {
            // printable us-ascii excluding `"`
            Regex::new(r#"^[[:print:]&&[^"]]{1,63}$"#).unwrap()
        });
        if RE.is_match(&identifier) {
            Ok(Self(identifier))
        } else {
            Err(InvalidQuotedIdentifier { identifier })
        }
    }
}

impl Display for QuotedIdentifier {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "\"{}\"", self.0)
    }
}

#[derive(Debug, Error)]
#[error("String is not a supported quoted identifier: {identifier:?}")]
pub(crate) struct InvalidQuotedIdentifier {
    identifier: String,
}

#[cfg(test)]
mod tests {
    use std::str;

    use super::*;

    #[test]
    fn test_quoted_identifier_parsing() {
        assert!(QuotedIdentifier::from_str("").is_err());
        assert!(QuotedIdentifier::from_str(str::from_utf8(&[0x41; 63]).unwrap()).is_ok());
        assert!(QuotedIdentifier::from_str(str::from_utf8(&[0x41; 64]).unwrap()).is_err());
        assert!(QuotedIdentifier::from_str("a").is_ok());
        for chr in ' '..='~' {
            assert_eq!(
                QuotedIdentifier::try_from(format!("{chr}")).is_ok(),
                chr != '"'
            );
        }
    }

    #[test]
    fn test_format_quoted_identifier() {
        assert_eq!(
            QuotedIdentifier::from_str("a").unwrap().to_string(),
            "\"a\""
        );
    }
}
