//! Diff checker feature.
//!
//! The core computes line-level diffs with word-level refinement for replaced
//! lines.  It exposes structured data for both the side-by-side view and the
//! Microsoft Word-style track-changes view, plus a plain unified diff helper
//! for the CLI.

use serde::{Deserialize, Serialize};
use similar::{DiffOp, TextDiff};
use thiserror::Error;

use crate::registry::ToolManifest;

pub const MANIFEST: ToolManifest = ToolManifest {
    id: "diff_checker",
    name: "Diff Checker",
    description: "Visualize text differences side-by-side or as Word-style track changes.",
    keywords: &["diff", "compare", "text", "track changes", "review"],
    version: "0.1.0",
};

/// Errors produced by diff processing.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum DiffError {
    #[error("diff input too large ({0} bytes); inputs over 50 MB are not supported")]
    InputTooLarge(usize),
}

/// Maximum combined input size for word-level diff (50 MB).
pub const MAX_DIFF_INPUT_BYTES: usize = 50 * 1024 * 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LineKind {
    Equal,
    Delete,
    Insert,
    Replace,
}

/// Byte ranges describing a changed word on both sides.
///
/// For delete-only words `right_start`/`right_len` are zero; for insert-only
/// words the left fields are zero.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct WordRange {
    pub left_start: usize,
    pub left_len: usize,
    pub right_start: usize,
    pub right_len: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SideBySideLine {
    pub line_number_left: Option<usize>,
    pub line_number_right: Option<usize>,
    pub left_text: String,
    pub right_text: String,
    pub kind: LineKind,
    pub word_diff: Vec<WordRange>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SideBySideDiff {
    pub lines: Vec<SideBySideLine>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TrackChangeKind {
    Equal,
    Inserted,
    Deleted,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TrackChangeSegment {
    pub text: String,
    pub kind: TrackChangeKind,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TrackChangesDiff {
    pub segments: Vec<TrackChangeSegment>,
}

fn normalize_line_endings(text: &str) -> std::borrow::Cow<'_, str> {
    if text.contains("\r\n") {
        std::borrow::Cow::Owned(text.replace("\r\n", "\n"))
    } else if text.contains('\r') {
        std::borrow::Cow::Owned(text.replace('\r', "\n"))
    } else {
        std::borrow::Cow::Borrowed(text)
    }
}

fn strip_line_ending(line: &str) -> (&str, Option<&'static str>) {
    if let Some(body) = line.strip_suffix('\n') {
        (body, Some("\n"))
    } else {
        (line, None)
    }
}

#[derive(Debug, Clone)]
struct Token {
    start: usize,
    end: usize,
    text: String,
}

fn is_cjk(ch: char) -> bool {
    matches!(ch,
        '\u{4E00}'..='\u{9FFF}'   // CJK Unified Ideographs
        | '\u{3400}'..='\u{4DBF}' // CJK Extension A
        | '\u{3040}'..='\u{30FF}' // Hiragana/Katakana
        | '\u{AC00}'..='\u{D7AF}' // Hangul syllables
        | '\u{F900}'..='\u{FAFF}' // CJK Compatibility Ideographs
        | '\u{20000}'..='\u{2A6DF}'
        | '\u{2A700}'..='\u{2B73F}'
        | '\u{2B740}'..='\u{2B81F}'
        | '\u{2B820}'..='\u{2CEAF}'
    )
}

/// Tokenize text into word-like tokens, keeping whitespace as separate tokens.
///
/// CJK characters are emitted as single-character tokens so single-character
/// differences are visible in Chinese/Japanese/Korean text.
fn tokenize(s: &str) -> Vec<Token> {
    let mut tokens = Vec::new();
    let mut i = 0;
    while i < s.len() {
        let ch = s[i..].chars().next().expect("in bounds");
        let ch_len = ch.len_utf8();
        let start = i;

        if ch.is_whitespace() {
            i += ch_len;
            while i < s.len() {
                let next = s[i..].chars().next().expect("in bounds");
                if next.is_whitespace() {
                    i += next.len_utf8();
                } else {
                    break;
                }
            }
        } else if is_cjk(ch) {
            i += ch_len;
        } else if ch.is_alphanumeric() || ch == '_' {
            i += ch_len;
            while i < s.len() {
                let next = s[i..].chars().next().expect("in bounds");
                if next.is_alphanumeric() || next == '_' {
                    i += next.len_utf8();
                } else {
                    break;
                }
            }
        } else {
            i += ch_len; // punctuation, symbols, combining marks -> one token
        }

        tokens.push(Token {
            start,
            end: i,
            text: s[start..i].to_string(),
        });
    }
    tokens
}

/// Word-level diff between two lines.
///
/// Uses `similar` on token sequences.  If the token count is very large the
/// line is treated as one replaced block to keep the operation sub-quadratic.
fn word_diff_ranges(left: &str, right: &str) -> Vec<WordRange> {
    let old_tokens = tokenize(left);
    let new_tokens = tokenize(right);

    // Fallback: a single coarse range instead of a very expensive token diff.
    if old_tokens.len().saturating_mul(new_tokens.len()) > 1_000_000 {
        return vec![WordRange {
            left_start: 0,
            left_len: left.len(),
            right_start: 0,
            right_len: right.len(),
        }];
    }

    let old_refs: Vec<&str> = old_tokens.iter().map(|t| t.text.as_str()).collect();
    let new_refs: Vec<&str> = new_tokens.iter().map(|t| t.text.as_str()).collect();
    let diff = TextDiff::from_slices(&old_refs, &new_refs);

    let mut ranges = Vec::new();
    for op in diff.ops() {
        match op {
            DiffOp::Equal { .. } => {}
            DiffOp::Delete {
                old_index, old_len, ..
            } => {
                if *old_len == 0 {
                    continue;
                }
                let start = old_tokens[*old_index].start;
                let end = old_tokens[old_index + old_len - 1].end;
                ranges.push(WordRange {
                    left_start: start,
                    left_len: end - start,
                    right_start: 0,
                    right_len: 0,
                });
            }
            DiffOp::Insert {
                new_index, new_len, ..
            } => {
                if *new_len == 0 {
                    continue;
                }
                let start = new_tokens[*new_index].start;
                let end = new_tokens[new_index + new_len - 1].end;
                ranges.push(WordRange {
                    left_start: 0,
                    left_len: 0,
                    right_start: start,
                    right_len: end - start,
                });
            }
            DiffOp::Replace {
                old_index,
                old_len,
                new_index,
                new_len,
            } => {
                let left_start = old_tokens[*old_index].start;
                let left_end = old_tokens[old_index + old_len - 1].end;
                let right_start = new_tokens[*new_index].start;
                let right_end = new_tokens[new_index + new_len - 1].end;
                ranges.push(WordRange {
                    left_start,
                    left_len: left_end - left_start,
                    right_start,
                    right_len: right_end - right_start,
                });
            }
        }
    }
    ranges
}

/// Build track-change segments for one pair of changed lines.
fn word_diff_segments(left: &str, right: &str) -> Vec<TrackChangeSegment> {
    let old_tokens = tokenize(left);
    let new_tokens = tokenize(right);

    if old_tokens.len().saturating_mul(new_tokens.len()) > 1_000_000 {
        let mut segments = Vec::new();
        if !left.is_empty() {
            segments.push(TrackChangeSegment {
                text: left.to_string(),
                kind: TrackChangeKind::Deleted,
            });
        }
        if !right.is_empty() {
            segments.push(TrackChangeSegment {
                text: right.to_string(),
                kind: TrackChangeKind::Inserted,
            });
        }
        return segments;
    }

    let old_refs: Vec<&str> = old_tokens.iter().map(|t| t.text.as_str()).collect();
    let new_refs: Vec<&str> = new_tokens.iter().map(|t| t.text.as_str()).collect();
    let diff = TextDiff::from_slices(&old_refs, &new_refs);

    let mut segments = Vec::new();
    for op in diff.ops() {
        match op {
            DiffOp::Equal { old_index, len, .. } => {
                for token in &old_tokens[*old_index..*old_index + len] {
                    segments.push(TrackChangeSegment {
                        text: token.text.clone(),
                        kind: TrackChangeKind::Equal,
                    });
                }
            }
            DiffOp::Delete {
                old_index, old_len, ..
            } => {
                for token in &old_tokens[*old_index..*old_index + old_len] {
                    segments.push(TrackChangeSegment {
                        text: token.text.clone(),
                        kind: TrackChangeKind::Deleted,
                    });
                }
            }
            DiffOp::Insert {
                new_index, new_len, ..
            } => {
                for token in &new_tokens[*new_index..*new_index + new_len] {
                    segments.push(TrackChangeSegment {
                        text: token.text.clone(),
                        kind: TrackChangeKind::Inserted,
                    });
                }
            }
            DiffOp::Replace {
                old_index,
                old_len,
                new_index,
                new_len,
            } => {
                for token in &old_tokens[*old_index..*old_index + old_len] {
                    segments.push(TrackChangeSegment {
                        text: token.text.clone(),
                        kind: TrackChangeKind::Deleted,
                    });
                }
                for token in &new_tokens[*new_index..*new_index + new_len] {
                    segments.push(TrackChangeSegment {
                        text: token.text.clone(),
                        kind: TrackChangeKind::Inserted,
                    });
                }
            }
        }
    }
    segments
}

fn side_by_side_line(
    kind: LineKind,
    line_number_left: Option<usize>,
    line_number_right: Option<usize>,
    left_text: &str,
    right_text: &str,
    word_diff: Vec<WordRange>,
) -> SideBySideLine {
    let (left_text, _) = strip_line_ending(left_text);
    let (right_text, _) = strip_line_ending(right_text);
    SideBySideLine {
        line_number_left,
        line_number_right,
        left_text: left_text.to_string(),
        right_text: right_text.to_string(),
        kind,
        word_diff,
    }
}

/// Compute a structured side-by-side diff.
pub fn diff_side_by_side(left: &str, right: &str) -> SideBySideDiff {
    let left = normalize_line_endings(left);
    let right = normalize_line_endings(right);
    let diff = TextDiff::from_lines(&left, &right);
    let old_lines = diff.old_slices();
    let new_lines = diff.new_slices();

    let mut lines = Vec::new();
    for op in diff.ops() {
        match *op {
            DiffOp::Equal {
                old_index,
                new_index,
                len,
            } => {
                for i in 0..len {
                    let text = old_lines[old_index + i];
                    lines.push(side_by_side_line(
                        LineKind::Equal,
                        Some(old_index + i + 1),
                        Some(new_index + i + 1),
                        text,
                        text,
                        Vec::new(),
                    ));
                }
            }
            DiffOp::Delete {
                old_index, old_len, ..
            } => {
                for i in 0..old_len {
                    let text = old_lines[old_index + i];
                    lines.push(side_by_side_line(
                        LineKind::Delete,
                        Some(old_index + i + 1),
                        None,
                        text,
                        "",
                        Vec::new(),
                    ));
                }
            }
            DiffOp::Insert {
                new_index, new_len, ..
            } => {
                for i in 0..new_len {
                    let text = new_lines[new_index + i];
                    lines.push(side_by_side_line(
                        LineKind::Insert,
                        None,
                        Some(new_index + i + 1),
                        "",
                        text,
                        Vec::new(),
                    ));
                }
            }
            DiffOp::Replace {
                old_index,
                old_len,
                new_index,
                new_len,
            } => {
                let paired = old_len.min(new_len);
                for i in 0..paired {
                    let old_line = old_lines[old_index + i];
                    let new_line = new_lines[new_index + i];
                    let (old_body, _) = strip_line_ending(old_line);
                    let (new_body, _) = strip_line_ending(new_line);
                    let word_diff = word_diff_ranges(old_body, new_body);
                    lines.push(side_by_side_line(
                        LineKind::Replace,
                        Some(old_index + i + 1),
                        Some(new_index + i + 1),
                        old_line,
                        new_line,
                        word_diff,
                    ));
                }
                for i in paired..old_len {
                    let text = old_lines[old_index + i];
                    lines.push(side_by_side_line(
                        LineKind::Delete,
                        Some(old_index + i + 1),
                        None,
                        text,
                        "",
                        Vec::new(),
                    ));
                }
                for i in paired..new_len {
                    let text = new_lines[new_index + i];
                    lines.push(side_by_side_line(
                        LineKind::Insert,
                        None,
                        Some(new_index + i + 1),
                        "",
                        text,
                        Vec::new(),
                    ));
                }
            }
        }
    }

    SideBySideDiff { lines }
}

/// Compute a Word-style track-changes diff.
///
/// The invariant is: applying the insertions and deleting the deletions to
/// `left` reconstructs `right`.
pub fn diff_track_changes(left: &str, right: &str) -> TrackChangesDiff {
    let left = normalize_line_endings(left);
    let right = normalize_line_endings(right);
    let diff = TextDiff::from_lines(&left, &right);
    let old_lines = diff.old_slices();
    let new_lines = diff.new_slices();

    let mut segments = Vec::new();
    for op in diff.ops() {
        match *op {
            DiffOp::Equal { old_index, len, .. } => {
                for i in 0..len {
                    segments.push(TrackChangeSegment {
                        text: old_lines[old_index + i].to_string(),
                        kind: TrackChangeKind::Equal,
                    });
                }
            }
            DiffOp::Delete {
                old_index, old_len, ..
            } => {
                for i in 0..old_len {
                    segments.push(TrackChangeSegment {
                        text: old_lines[old_index + i].to_string(),
                        kind: TrackChangeKind::Deleted,
                    });
                }
            }
            DiffOp::Insert {
                new_index, new_len, ..
            } => {
                for i in 0..new_len {
                    segments.push(TrackChangeSegment {
                        text: new_lines[new_index + i].to_string(),
                        kind: TrackChangeKind::Inserted,
                    });
                }
            }
            DiffOp::Replace {
                old_index,
                old_len,
                new_index,
                new_len,
            } => {
                let paired = old_len.min(new_len);
                for i in 0..paired {
                    let old_line = old_lines[old_index + i];
                    let new_line = new_lines[new_index + i];
                    let (old_body, old_end) = strip_line_ending(old_line);
                    let (new_body, new_end) = strip_line_ending(new_line);

                    let mut line_segments = word_diff_segments(old_body, new_body);
                    // Attach the newline (if any) to the final segment so
                    // line breaks survive a render pass.
                    if let Some(end) = old_end.or(new_end) {
                        if let Some(last) = line_segments.last_mut() {
                            last.text.push_str(end);
                        } else {
                            line_segments.push(TrackChangeSegment {
                                text: end.to_string(),
                                kind: TrackChangeKind::Equal,
                            });
                        }
                    }
                    segments.extend(line_segments);
                }
                for i in paired..old_len {
                    segments.push(TrackChangeSegment {
                        text: old_lines[old_index + i].to_string(),
                        kind: TrackChangeKind::Deleted,
                    });
                }
                for i in paired..new_len {
                    segments.push(TrackChangeSegment {
                        text: new_lines[new_index + i].to_string(),
                        kind: TrackChangeKind::Inserted,
                    });
                }
            }
        }
    }

    TrackChangesDiff { segments }
}

/// Plain unified diff, useful for CLI `--format unified`.
pub fn diff_unified(left: &str, right: &str) -> String {
    let left = normalize_line_endings(left);
    let right = normalize_line_endings(right);
    let diff = TextDiff::from_lines(&left, &right);
    diff.unified_diff()
        .context_radius(3)
        .header("left", "right")
        .to_string()
}

/// Return true when normalized inputs are identical.
pub fn is_identical(left: &str, right: &str) -> bool {
    normalize_line_endings(left) == normalize_line_endings(right)
}

/// Return a suggested "input too large" error if needed.
pub fn validate_input_size(left: &str, right: &str) -> Result<(), DiffError> {
    if left.len().saturating_add(right.len()) > MAX_DIFF_INPUT_BYTES {
        return Err(DiffError::InputTooLarge(
            left.len().saturating_add(right.len()),
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn left_texts(diff: &SideBySideDiff) -> Vec<&str> {
        diff.lines.iter().map(|l| l.left_text.as_str()).collect()
    }

    #[test]
    fn identical_inputs_are_all_equal() {
        let d = diff_side_by_side("a\nb\n", "a\nb\n");
        assert_eq!(d.lines.len(), 2);
        assert!(d.lines.iter().all(|l| l.kind == LineKind::Equal));
    }

    #[test]
    fn insert_and_delete_lines() {
        let d = diff_side_by_side("a\nb\n", "a\nb\nc\n");
        assert_eq!(
            d.lines
                .iter()
                .filter(|l| l.kind == LineKind::Insert)
                .count(),
            1
        );
        assert_eq!(left_texts(&d), vec!["a", "b", ""]);

        let d = diff_side_by_side("a\nb\nc\n", "a\n");
        assert_eq!(
            d.lines
                .iter()
                .filter(|l| l.kind == LineKind::Delete)
                .count(),
            2
        );
    }

    #[test]
    fn replace_line_has_word_diff_only_for_changed_word() {
        let d = diff_side_by_side("The quick brown fox\n", "The quick red fox\n");
        assert_eq!(d.lines.len(), 1);
        assert_eq!(d.lines[0].kind, LineKind::Replace);
        assert_eq!(d.lines[0].word_diff.len(), 1);
        let wr = d.lines[0].word_diff[0];
        assert_eq!(
            &d.lines[0].left_text[wr.left_start..wr.left_start + wr.left_len],
            "brown"
        );
        assert_eq!(
            &d.lines[0].right_text[wr.right_start..wr.right_start + wr.right_len],
            "red"
        );
    }

    #[test]
    fn cjk_word_diff_is_character_level() {
        let d = diff_side_by_side("你好世界\n", "你好地球\n");
        assert_eq!(d.lines[0].kind, LineKind::Replace);
        assert_eq!(d.lines[0].word_diff.len(), 1);
        let wr = d.lines[0].word_diff[0];
        assert_eq!(
            &d.lines[0].left_text[wr.left_start..wr.left_start + wr.left_len],
            "世界"
        );
        assert_eq!(
            &d.lines[0].right_text[wr.right_start..wr.right_start + wr.right_len],
            "地球"
        );
    }

    #[test]
    fn crlf_is_normalized() {
        let d = diff_side_by_side("a\r\nb\r\n", "a\nb\n");
        assert!(d.lines.iter().all(|l| l.kind == LineKind::Equal));
    }

    #[test]
    fn track_changes_reconstructs_right() {
        let left = "a\nb\nc\n";
        let right = "a\nB\nc\nd\n";
        let tc = diff_track_changes(left, right);
        let mut reconstructed = String::new();
        for seg in &tc.segments {
            if seg.kind != TrackChangeKind::Deleted {
                reconstructed.push_str(&seg.text);
            }
        }
        assert_eq!(reconstructed, right);
    }

    #[test]
    fn unified_diff_contains_hunks() {
        let u = diff_unified("a\nb\n", "a\nc\n");
        assert!(u.contains("@@"));
        assert!(u.contains("-b"));
        assert!(u.contains("+c"));
    }
}
