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

/// Clean a query from from symbols and multiple spaces.
pub fn clean_query(query: impl AsRef<str>) -> String {
    use once_cell::sync::Lazy;
    use regex::Regex;

    /// match any sequence of symbols and spaces that can follow
    static SYMBOLS: Lazy<Regex> =
        Lazy::new(|| Regex::new(r"[\p{Symbol}\p{Punctuation}]+\p{Separator}*").unwrap());
    /// match any sequence spaces
    static SEPARATORS: Lazy<Regex> = Lazy::new(|| Regex::new(r"\p{Separator}+").unwrap());

    // we replace a symbol with a space
    let no_symbols = SYMBOLS.replace_all(query.as_ref(), " ");
    // we collapse sequence of spaces to only one
    SEPARATORS
        .replace_all(&no_symbols, " ")
        .trim()
        .to_lowercase()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_symbol_is_identity_letters() {
        let s = "aàáâäąbßcçdeèéêëęfghiìíîïlłmnǹńoòóôöpqrsśtuùúüvwyỳýÿzź";
        assert_eq!(clean_query(s), s);
    }

    #[test]
    fn no_symbol_is_identity_numbers() {
        let s = "0123456789";
        assert_eq!(clean_query(s), s);
    }

    #[test]
    fn remove_symbols() {
        assert_eq!(clean_query("!$\",?(){};:."), "");
    }

    #[test]
    fn remove_symbols_adjust_space_between() {
        for s in ["a-b", "a - b"] {
            assert_eq!(clean_query(s), "a b");
        }
    }

    #[test]
    fn remove_symbols_adjust_space_after() {
        for s in ["a!  ", "a ! ", "a  !  "] {
            assert_eq!(clean_query(s), "a");
        }
    }

    #[test]
    fn remove_symbols_adjust_space_before() {
        for s in ["  !a ", " ! a ", "  !  a  "] {
            assert_eq!(clean_query(s), "a");
        }
    }

    #[test]
    fn adjust_spaces() {
        assert_eq!(clean_query("  a  b  c  "), "a b c");
    }

    #[test]
    fn lowercase_query() {
        assert_eq!(clean_query("AND OR Or aNd"), "and or or and");
    }
}
