//! Case converter feature.
//!
//! Provides five text conversions: sentence case, lower case, UPPER CASE,
//! Capitalized Case, and Title Case.  Sentence case supports a user-supplied
//! proper-noun list.  The conversion is infallible; malformed or unusual text
//! is normalized as gracefully as the rules allow.

use serde::{Deserialize, Serialize};
use unicode_segmentation::UnicodeSegmentation;

use crate::registry::ToolManifest;

pub const MANIFEST: ToolManifest = ToolManifest {
    id: "case_converter",
    name: "Case Converter",
    description:
        "Convert text to sentence case, lower case, UPPER CASE, Capitalized Case, or Title Case.",
    keywords: &[
        "case",
        "convert",
        "sentence",
        "title",
        "upper",
        "lower",
        "capitalize",
        "text",
    ],
    version: "0.1.0",
};

/// Conversion mode selected by the caller.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CaseMode {
    Sentence,
    Lower,
    Upper,
    Capitalized,
    Title,
}

/// Minor words kept lowercase in the middle of a Title Case line.
///
/// The list contains articles, conjunctions, and short prepositions of three
/// letters or fewer, so the long-word rule never conflicts with it.
pub const TITLE_MINOR_WORDS: &[&str] = &[
    "a", "an", "and", "as", "at", "but", "by", "for", "if", "in", "nor", "of", "off", "on", "or",
    "per", "so", "the", "to", "up", "via", "vs", "yet",
];

/// Words with at least this many alphabetic letters are always capitalized in
/// Title Case, even if they would otherwise be treated as minor words.
pub const TITLE_LONG_WORD_MIN_CHARS: usize = 4;

const SENTENCE_ABBREVIATIONS: &[&str] = &[
    "mr", "mrs", "ms", "dr", "prof", "st", "vs", "etc", "e.g", "i.e",
];

/// Convert `text` using `mode`.
///
/// `proper_nouns` is used by [`CaseMode::Sentence`] and [`CaseMode::Title`].
/// The conversion is infallible: every input string is valid.
pub fn convert(text: &str, mode: CaseMode, proper_nouns: &[String]) -> String {
    match mode {
        CaseMode::Sentence => sentence_case(text, proper_nouns),
        CaseMode::Lower => lower_case(text),
        CaseMode::Upper => upper_case(text),
        CaseMode::Capitalized => capitalized_case(text),
        CaseMode::Title => title_case(text, proper_nouns),
    }
}

/// Convert to sentence case.
///
/// Each sentence has its first word capitalized.  User-supplied proper nouns
/// are matched case-insensitively as whole words or whitespace-separated
/// phrases and replaced with the spelling supplied by the user.
pub fn sentence_case(text: &str, proper_nouns: &[String]) -> String {
    let mut converted = String::with_capacity(text.len());
    for segment in sentence_segments(text) {
        converted.push_str(&capitalize_first_word_in_sentence(segment));
    }

    if proper_nouns.is_empty() {
        return converted;
    }

    apply_proper_nouns(&converted, proper_nouns)
}

/// Convert every letter to lowercase.
pub fn lower_case(text: &str) -> String {
    text.to_lowercase()
}

/// Convert every letter to uppercase.
pub fn upper_case(text: &str) -> String {
    text.to_uppercase()
}

/// Capitalize the first letter of every word.
pub fn capitalized_case(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for token in text.split_word_bounds() {
        if contains_cased_char(token) {
            out.push_str(&capitalize_first_cased(token));
        } else {
            out.push_str(token);
        }
    }
    out
}

/// Convert each non-empty line to Title Case.
///
/// The first and last cased word of a line are always capitalized.  Minor
/// words stay lowercase in the middle unless they follow a colon or dash or
/// are long words.  User-supplied proper nouns are applied after the title
/// rules and replace matched words with the user's exact spelling.
pub fn title_case(text: &str, proper_nouns: &[String]) -> String {
    let mut converted = String::with_capacity(text.len());
    for line in split_lines_inclusive(text) {
        converted.push_str(&title_case_line(line));
    }

    if proper_nouns.is_empty() {
        return converted;
    }

    apply_proper_nouns(&converted, proper_nouns)
}

fn title_case_line(line: &str) -> String {
    let word_count = line
        .split_word_bound_indices()
        .filter(|(_, segment)| contains_cased_char(segment))
        .count();
    if word_count == 0 {
        return line.to_string();
    }

    let last_word_index = word_count - 1;
    let mut out = String::with_capacity(line.len());
    let mut last_end = 0usize;
    let mut word_index = 0usize;

    for (start, segment) in line.split_word_bound_indices() {
        out.push_str(&line[last_end..start]);

        if contains_cased_char(segment) {
            let lower = segment.to_lowercase();
            let is_first = word_index == 0;
            let is_last = word_index == last_word_index;
            let starts_new_section = follows_title_break(&line[..start]);

            if is_first || is_last || starts_new_section {
                out.push_str(&capitalize_first_cased(segment));
            } else if TITLE_MINOR_WORDS.contains(&lower.as_str())
                && count_alphabetic(&lower) < TITLE_LONG_WORD_MIN_CHARS
            {
                out.push_str(&lower);
            } else {
                out.push_str(&capitalize_first_cased(segment));
            }

            word_index += 1;
        } else {
            out.push_str(segment);
        }

        last_end = start + segment.len();
    }

    out.push_str(&line[last_end..]);
    out
}

fn follows_title_break(prefix: &str) -> bool {
    let trimmed = prefix.trim_end_matches(|ch: char| {
        ch.is_whitespace() || matches!(ch, '"' | '\'' | '“' | '”' | '‘' | '’' | '(' | '[' | '{')
    });
    trimmed.ends_with(':') || trimmed.ends_with('—') || trimmed.ends_with('–')
}

fn capitalize_first_cased(token: &str) -> String {
    let lower = token.to_lowercase();
    if is_ordinal_suffix(&lower) {
        return lower;
    }

    let mut out = String::with_capacity(lower.len());
    let mut capitalized = false;

    for ch in lower.chars() {
        if !capitalized && is_cased(ch) {
            out.extend(ch.to_uppercase());
            capitalized = true;
        } else {
            out.push(ch);
        }
    }

    out
}

fn is_ordinal_suffix(token: &str) -> bool {
    let digit_count = token.chars().take_while(|ch| ch.is_ascii_digit()).count();
    if digit_count == 0 || digit_count == token.chars().count() {
        return false;
    }

    let suffix: String = token.chars().skip(digit_count).collect();
    matches!(suffix.as_str(), "st" | "nd" | "rd" | "th")
}

fn capitalize_first_word_in_sentence(segment: &str) -> String {
    let lowered = segment.to_lowercase();
    let mut out = String::with_capacity(lowered.len());
    let mut first_word_done = false;

    for token in lowered.split_word_bounds() {
        if !first_word_done && token.chars().any(|ch| ch.is_alphanumeric()) {
            if contains_cased_char(token) {
                out.push_str(&capitalize_first_cased(token));
            } else {
                out.push_str(token);
            }
            first_word_done = true;
        } else {
            out.push_str(token);
        }
    }

    out
}

fn sentence_segments(text: &str) -> Vec<&str> {
    let chars: Vec<(usize, char)> = text.char_indices().collect();
    let mut starts = vec![0usize];
    let mut index = 0usize;

    while index < chars.len() {
        let (byte_index, ch) = chars[index];

        if ch == '\n' {
            let end = byte_index + ch.len_utf8();
            if end < text.len() {
                starts.push(end);
            }
            index += 1;
            continue;
        }

        if ch == '\r' {
            let end = byte_index + ch.len_utf8();
            let next_is_lf = chars.get(index + 1).is_some_and(|(_, next)| *next == '\n');
            if !next_is_lf && end < text.len() {
                starts.push(end);
            }
            index += 1;
            continue;
        }

        if !is_sentence_terminator(ch) {
            index += 1;
            continue;
        }

        let run_start = byte_index;
        let mut run_end_index = index;
        let mut period_count = 0usize;
        let mut has_non_period = false;

        while run_end_index < chars.len() && is_sentence_terminator(chars[run_end_index].1) {
            if chars[run_end_index].1 == '.' {
                period_count += 1;
            } else {
                has_non_period = true;
            }
            run_end_index += 1;
        }

        let last_terminator = chars[run_end_index - 1];
        let run_end = last_terminator.0 + last_terminator.1.len_utf8();

        let mut next_index = run_end_index;
        while next_index < chars.len()
            && (is_sentence_closer(chars[next_index].1) || chars[next_index].1.is_whitespace())
        {
            next_index += 1;
        }

        if next_index < chars.len() && chars[next_index].1.is_alphabetic() {
            let suppress = if ch == '.' {
                is_decimal_terminator(text, run_start, run_end)
                    || (period_count == 1
                        && !has_non_period
                        && is_abbreviation_before(text, run_start))
            } else {
                false
            };

            if !suppress {
                starts.push(chars[next_index].0);
            }
        }

        index = run_end_index;
    }

    starts.sort_unstable();
    starts.dedup();

    let mut segments = Vec::with_capacity(starts.len());
    for (position, start) in starts.iter().enumerate() {
        let end = starts.get(position + 1).copied().unwrap_or(text.len());
        if *start < end {
            segments.push(&text[*start..end]);
        }
    }

    segments
}

fn is_sentence_terminator(ch: char) -> bool {
    matches!(ch, '.' | '!' | '?' | '…' | '。' | '！' | '？')
}

fn is_sentence_closer(ch: char) -> bool {
    matches!(
        ch,
        '"' | '\'' | '”' | '’' | '»' | ')' | ']' | '}' | '》' | '）'
    )
}

fn is_decimal_terminator(text: &str, run_start: usize, run_end: usize) -> bool {
    let before = text[..run_start].chars().next_back();
    let after = text[run_end..].chars().next();
    before.is_some_and(|ch| ch.is_numeric()) && after.is_some_and(|ch| ch.is_numeric())
}

fn is_abbreviation_before(text: &str, run_start: usize) -> bool {
    let token = token_before(text, run_start);
    let lower = token.to_lowercase();
    SENTENCE_ABBREVIATIONS.contains(&lower.as_str())
        || token.contains('.')
        || (token.chars().count() == 1 && token.chars().next().is_some_and(|ch| ch.is_alphabetic()))
}

fn token_before(text: &str, byte_index: usize) -> &str {
    let prefix = &text[..byte_index];
    let trimmed = prefix.trim_end_matches(|ch: char| ch.is_whitespace() || is_sentence_closer(ch));

    let start = trimmed
        .char_indices()
        .rev()
        .take_while(|(_, ch)| ch.is_alphanumeric() || matches!(ch, '.' | '\'' | '’' | '`'))
        .last()
        .map(|(index, _)| index)
        .unwrap_or(trimmed.len());

    &trimmed[start..]
}

fn split_lines_inclusive(text: &str) -> Vec<&str> {
    let chars: Vec<(usize, char)> = text.char_indices().collect();
    let mut lines = Vec::new();
    let mut start = 0usize;
    let mut index = 0usize;

    while index < chars.len() {
        let (byte_index, ch) = chars[index];

        if ch == '\n' {
            let end = byte_index + ch.len_utf8();
            lines.push(&text[start..end]);
            start = end;
        } else if ch == '\r' {
            let next_is_lf = chars.get(index + 1).is_some_and(|(_, next)| *next == '\n');
            if !next_is_lf {
                let end = byte_index + ch.len_utf8();
                lines.push(&text[start..end]);
                start = end;
            }
        }

        index += 1;
    }

    if start < text.len() {
        lines.push(&text[start..]);
    }

    lines
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ProperNoun {
    words: Vec<String>,
    keys: Vec<String>,
}

fn normalized_proper_nouns(proper_nouns: &[String]) -> Vec<ProperNoun> {
    let mut normalized: Vec<ProperNoun> = Vec::new();

    for raw in proper_nouns {
        let words: Vec<&str> = raw.unicode_words().collect();
        if words.is_empty() {
            continue;
        }

        let keys: Vec<String> = words.iter().map(|word| word.to_lowercase()).collect();
        if normalized.iter().any(|noun| noun.keys == keys) {
            continue;
        }

        normalized.push(ProperNoun {
            words: words.into_iter().map(str::to_string).collect(),
            keys,
        });
    }

    normalized.sort_by(|left, right| {
        right
            .keys
            .len()
            .cmp(&left.keys.len())
            .then_with(|| total_key_len(right).cmp(&total_key_len(left)))
    });

    normalized
}

fn total_key_len(noun: &ProperNoun) -> usize {
    noun.keys.iter().map(String::len).sum()
}

fn apply_proper_nouns(text: &str, proper_nouns: &[String]) -> String {
    let nouns = normalized_proper_nouns(proper_nouns);
    if nouns.is_empty() {
        return text.to_string();
    }

    let tokens: Vec<(usize, &str)> = text
        .split_word_bound_indices()
        .filter(|(_, segment)| segment.chars().any(|ch| ch.is_alphabetic()))
        .collect();

    if tokens.is_empty() {
        return text.to_string();
    }

    let mut out = String::with_capacity(text.len());
    let mut cursor = 0usize;
    let mut token_index = 0usize;

    while token_index < tokens.len() {
        let matched = nouns.iter().find(|noun| {
            if token_index + noun.words.len() > tokens.len() {
                return false;
            }

            noun.keys.iter().enumerate().all(|(offset, key)| {
                if tokens[token_index + offset].1.to_lowercase() != *key {
                    return false;
                }

                if offset + 1 < noun.words.len() {
                    let separator_start =
                        tokens[token_index + offset].0 + tokens[token_index + offset].1.len();
                    let separator_end = tokens[token_index + offset + 1].0;
                    return text[separator_start..separator_end]
                        .chars()
                        .all(char::is_whitespace);
                }

                true
            })
        });

        if let Some(noun) = matched {
            out.push_str(&text[cursor..tokens[token_index].0]);

            for (offset, word) in noun.words.iter().enumerate() {
                out.push_str(word);

                if offset + 1 < noun.words.len() {
                    let separator_start =
                        tokens[token_index + offset].0 + tokens[token_index + offset].1.len();
                    let separator_end = tokens[token_index + offset + 1].0;
                    out.push_str(&text[separator_start..separator_end]);
                }
            }

            let last_offset = noun.words.len() - 1;
            cursor =
                tokens[token_index + last_offset].0 + tokens[token_index + last_offset].1.len();
            token_index += noun.words.len();
        } else {
            token_index += 1;
        }
    }

    out.push_str(&text[cursor..]);
    out
}

fn is_cased(ch: char) -> bool {
    ch.is_lowercase() || ch.is_uppercase()
}

fn contains_cased_char(text: &str) -> bool {
    text.chars().any(is_cased)
}

fn count_alphabetic(text: &str) -> usize {
    text.chars().filter(|ch| ch.is_alphabetic()).count()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn nouns(values: &[&str]) -> Vec<String> {
        values.iter().map(|value| value.to_string()).collect()
    }

    #[test]
    fn lower_and_upper_are_unicode_aware() {
        assert_eq!(lower_case("Hello, WORLD!"), "hello, world!");
        assert_eq!(upper_case("Straße"), "STRASSE");
        assert_eq!(lower_case(""), "");
        assert_eq!(upper_case(""), "");
    }

    #[test]
    fn sentence_case_capitalizes_each_sentence() {
        assert_eq!(
            sentence_case("hello world. this is a test!", &[]),
            "Hello world. This is a test!"
        );
        assert_eq!(
            sentence_case("HELLO WORLD. THIS IS A TEST!", &[]),
            "Hello world. This is a test!"
        );
        assert_eq!(sentence_case("what?! really.", &[]), "What?! Really.");
        assert_eq!(sentence_case("hello\nworld", &[]), "Hello\nWorld");
        assert_eq!(sentence_case("hello\r\nworld", &[]), "Hello\r\nWorld");
    }

    #[test]
    fn sentence_case_handles_quotes_and_leading_punctuation() {
        assert_eq!(
            sentence_case("\"hello,\" she said.", &[]),
            "\"Hello,\" she said."
        );
        assert_eq!(sentence_case("'hello'", &[]), "'Hello'");
    }

    #[test]
    fn sentence_case_skips_first_words_without_letters() {
        assert_eq!(sentence_case("123 hello world", &[]), "123 hello world");
        assert_eq!(sentence_case("3.14 is pi", &[]), "3.14 is pi");
        assert_eq!(sentence_case("4th place is nice", &[]), "4th place is nice");
    }

    #[test]
    fn ordinal_suffixes_are_not_capitalized() {
        assert_eq!(capitalized_case("4th place"), "4th Place");
        assert_eq!(capitalized_case("21ST century"), "21st Century");
        assert_eq!(title_case("4th place", &[]), "4th Place");
    }

    #[test]
    fn sentence_case_respects_common_abbreviations() {
        assert_eq!(
            sentence_case("e.g. this is an example", &[]),
            "E.g. this is an example"
        );
        assert_eq!(
            sentence_case("Dr. smith went home.", &[]),
            "Dr. smith went home."
        );
    }

    #[test]
    fn sentence_case_applies_proper_nouns() {
        assert_eq!(
            sentence_case(
                "john met mary in new york",
                &nouns(&["John", "Mary", "New York"])
            ),
            "John met Mary in New York"
        );
        assert_eq!(
            sentence_case("iphone and nasa are here", &nouns(&["iPhone", "NASA"])),
            "iPhone and NASA are here"
        );
        assert_eq!(
            sentence_case("JOHN and jOhN", &nouns(&["John"])),
            "John and John"
        );
        assert_eq!(
            sentence_case("u.s.a. is big.", &nouns(&["U.S.A."])),
            "U.S.A. is big."
        );
        assert_eq!(
            sentence_case("o'brien called.", &nouns(&["O'Brien"])),
            "O'Brien called."
        );
    }

    #[test]
    fn sentence_case_proper_nouns_are_whole_words() {
        assert_eq!(
            sentence_case("ann planning annex", &nouns(&["Ann"])),
            "Ann planning annex"
        );
        assert_eq!(
            sentence_case("the annex is planned", &nouns(&["Ann"])),
            "The annex is planned"
        );
    }

    #[test]
    fn sentence_case_prefers_longest_proper_noun() {
        assert_eq!(
            sentence_case("new york city", &nouns(&["New", "New York"])),
            "New York city"
        );
    }

    #[test]
    fn sentence_case_ignores_blank_and_duplicate_nouns() {
        assert_eq!(
            sentence_case("john and john", &nouns(&["", "John", "john"])),
            "John and John"
        );
    }

    #[test]
    fn capitalized_case_capitalizes_every_word() {
        assert_eq!(capitalized_case("hello WORLD"), "Hello World");
        assert_eq!(
            capitalized_case("don't state-of-the-art"),
            "Don't State-Of-The-Art"
        );
        assert_eq!(capitalized_case("3d printing"), "3D Printing");
        assert_eq!(capitalized_case("éclair"), "Éclair");
        assert_eq!(capitalized_case("!!!"), "!!!");
    }

    #[test]
    fn title_case_keeps_minor_words_lowercase() {
        assert_eq!(
            title_case("the lord of the rings", &[]),
            "The Lord of the Rings"
        );
        assert_eq!(
            title_case("a tale of two cities", &[]),
            "A Tale of Two Cities"
        );
        assert_eq!(title_case("war and peace", &[]), "War and Peace");
        assert_eq!(
            title_case("to kill a mockingbird", &[]),
            "To Kill a Mockingbird"
        );
        assert_eq!(title_case("of mice and men", &[]), "Of Mice and Men");
        assert_eq!(
            title_case("something to believe in", &[]),
            "Something to Believe In"
        );
        assert_eq!(title_case("the end", &[]), "The End");
    }

    #[test]
    fn title_case_capitalizes_after_colon_and_dash() {
        assert_eq!(
            title_case("star wars: a new hope", &[]),
            "Star Wars: A New Hope"
        );
        assert_eq!(
            title_case("life — a user's manual", &[]),
            "Life — A User's Manual"
        );
    }

    #[test]
    fn title_case_handles_long_words_and_hyphenated_words() {
        assert_eq!(
            title_case("the state-of-the-art method", &[]),
            "The State-of-the-Art Method"
        );
        assert_eq!(
            title_case("an introduction to rust", &[]),
            "An Introduction to Rust"
        );
        assert_eq!(
            title_case("with great power comes great responsibility", &[]),
            "With Great Power Comes Great Responsibility"
        );
    }

    #[test]
    fn title_case_normalizes_all_caps_input() {
        assert_eq!(
            title_case("THE LORD OF THE RINGS", &[]),
            "The Lord of the Rings"
        );
        assert_eq!(
            title_case("NASA and the space race", &[]),
            "Nasa and the Space Race"
        );
    }

    #[test]
    fn title_case_applies_proper_nouns() {
        assert_eq!(
            title_case("nasa and the space race", &nouns(&["NASA"])),
            "NASA and the Space Race"
        );
        assert_eq!(
            title_case("the iphone era", &nouns(&["iPhone"])),
            "The iPhone Era"
        );
        assert_eq!(
            title_case("the lord of the rings", &nouns(&["Rings"])),
            "The Lord of the Rings"
        );
    }

    #[test]
    fn title_case_treats_each_line_as_a_title() {
        assert_eq!(
            title_case("the lord of the rings\nof mice and men", &[]),
            "The Lord of the Rings\nOf Mice and Men"
        );
        assert_eq!(title_case("123 456", &[]), "123 456");
        assert_eq!(title_case("", &[]), "");
    }

    #[test]
    fn dispatch_uses_selected_mode() {
        assert_eq!(
            convert("hello world", CaseMode::Capitalized, &[]),
            "Hello World"
        );
        assert_eq!(convert("hello world", CaseMode::Upper, &[]), "HELLO WORLD");
        assert_eq!(
            convert("hello world", CaseMode::Sentence, &nouns(&["World"])),
            "Hello World"
        );
        assert_eq!(
            convert(
                "nasa and the space race",
                CaseMode::Title,
                &nouns(&["NASA"])
            ),
            "NASA and the Space Race"
        );
    }
}
