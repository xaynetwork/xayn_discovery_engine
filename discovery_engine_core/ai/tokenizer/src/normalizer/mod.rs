pub mod pattern;
pub mod string;

use unicode_categories::UnicodeCategories;

use crate::normalizer::string::NormalizedString;

/// Whether or not to cleanse control characters.
#[derive(Clone, Copy, Debug)]
pub enum ControlChars {
    /// Keeps control characters.
    Keep,
    /// Cleanses control characters.
    Cleanse,
}

/// Whether or not to separate chinese characters.
#[derive(Clone, Copy, Debug)]
pub enum ChineseChars {
    /// Keeps chinese characters as given.
    Keep,
    /// Separates chinese characters with whitespace.
    Separate,
}

/// Whether or not to cleanse accents.
#[derive(Clone, Copy, Debug)]
pub enum AccentChars {
    /// Keeps accents as given.
    Keep,
    /// Cleanses accents from characters.
    Cleanse,
}

/// Whether or not to lowercase characters.
#[derive(Clone, Copy, Debug)]
pub enum CaseChars {
    /// Keeps case of characters as given.
    Keep,
    /// Lowercases characters.
    Lower,
}

/// A Bert normalizer.
#[derive(Debug)]
pub struct Normalizer {
    control: ControlChars,
    chinese: ChineseChars,
    accents: AccentChars,
    case: CaseChars,
}

impl Normalizer {
    /// Creates a Bert normalizer.
    pub(crate) fn new(
        control: ControlChars,
        chinese: ChineseChars,
        accents: AccentChars,
        case: CaseChars,
    ) -> Self {
        Self {
            control,
            chinese,
            accents,
            case,
        }
    }

    /// Cleans the sequence from control characters.
    fn clean(&self, sequence: NormalizedString) -> NormalizedString {
        if let ControlChars::Cleanse = self.control {
            sequence
                .filter(|c| {
                    c != '\0'
                        && c != '\u{fffd}'
                        && (c == '\t' || c == '\n' || c == '\r' || !c.is_other())
                })
                .map(|c| {
                    // These are technically control characters but we count them as whitespace
                    // The definition of `is_control` here is quite large and contains also
                    // Cc, Cf, Cn or Co; cf. https://unicode.org/reports/tr44/ (Table 12)
                    if c == '\t' || c == '\n' || c == '\r' || c.is_whitespace() {
                        ' '
                    } else {
                        c
                    }
                })
        } else {
            sequence
        }
    }

    /// Separates Chinese characters in the sequence by whitespace.
    fn separate_chinese(&self, sequence: NormalizedString) -> NormalizedString {
        if let ChineseChars::Separate = self.chinese {
            let mut new_chars: Vec<(char, isize)> = vec![];
            sequence.for_each_char(|c| {
                // Checks whether a character is Chinese
                // This defines a "Chinese character" as anything in the CJK Unicode block:
                //   https://en.wikipedia.org/wiki/CJK_Unified_Ideographs_(Unicode_block)
                //
                // Note that the CJK Unicode block is NOT all Japanese and Korean characters,
                // despite its name. The modern Korean Hangul alphabet is a different block,
                // as is Japanese Hiragana and Katakana. Those alphabets are used to write
                // space-separated words, so they are not treated specially and handled
                // like for all of the other languages.
                if let '\u{4E00}'..='\u{9FFF}'
                | '\u{3400}'..='\u{4DBF}'
                | '\u{20000}'..='\u{2A6DF}'
                | '\u{2A700}'..='\u{2B73F}'
                | '\u{2B740}'..='\u{2B81F}'
                | '\u{2B920}'..='\u{2CEAF}'
                | '\u{F900}'..='\u{FAFF}'
                | '\u{2F800}'..='\u{2FA1F}' = c
                {
                    new_chars.extend(&[(' ', 0), (c, 1), (' ', 1)]);
                } else {
                    new_chars.push((c, 0));
                }
            });
            sequence.transform(new_chars, 0)
        } else {
            sequence
        }
    }

    /// Strips accents from the sequence.
    fn strip_accents(&self, sequence: NormalizedString) -> NormalizedString {
        if let AccentChars::Cleanse = self.accents {
            sequence.nfd().filter(|c| !c.is_mark_nonspacing())
        } else {
            sequence
        }
    }

    /// Lowercases the sequence.
    fn lowercase(&self, sequence: NormalizedString) -> NormalizedString {
        if let CaseChars::Lower = self.case {
            sequence.lowercase()
        } else {
            sequence
        }
    }

    /// Normalizes the sequence.
    pub(crate) fn normalize(&self, sequence: impl AsRef<str>) -> NormalizedString {
        let sequence = self.clean(sequence.into());
        let sequence = self.separate_chinese(sequence);
        let sequence = self.strip_accents(sequence);
        self.lowercase(sequence)
    }
}
