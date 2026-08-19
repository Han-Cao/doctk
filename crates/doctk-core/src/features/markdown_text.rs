//! Markdown → plain-text conversion feature.
//!
//! Removes common markdown syntax and turns it into readable plain text.
//! Link handling is intentional:
//! - URL and file targets are kept in parentheses.
//! - Internal section links (`#section`) have the target removed.

use crate::registry::ToolManifest;

pub const MANIFEST: ToolManifest = ToolManifest {
    id: "markdown_text",
    name: "Markdown → Text",
    description: "Remove markdown syntax and convert the document to plain text.",
    keywords: &["markdown", "text", "plain text", "convert", "strip"],
    version: "0.1.0",
};

const PLACEHOLDER_PREFIX: char = '\u{0}';

/// Convert markdown to plain text. The conversion is best-effort and does not
/// fail; malformed markdown is emitted as-is or after light normalization.
pub fn markdown_to_text(md: &str) -> String {
    let md = normalize_line_endings(md);
    let mut protected: Vec<String> = Vec::new();
    let mut lines: Vec<String> = Vec::new();
    let mut in_code_block = false;

    for line in md.split('\n') {
        let trimmed = line.trim();
        let fence_len = fence_length(trimmed);

        if !in_code_block {
            if fence_len.is_some() {
                in_code_block = true;
                continue;
            }
            if is_horizontal_rule(trimmed) {
                continue;
            }
            let line = remove_blockquote(&remove_heading(line));
            let line = remove_list_marker(&line);
            let line = process_inline(&line, &mut protected);
            lines.push(line.trim_end().to_string());
        } else {
            // Inside a fenced code block. Closing fence exits; content is kept
            // verbatim and must not be processed as inline markdown.
            if fence_len.is_some() {
                in_code_block = false;
                continue;
            }
            lines.push(line.to_string());
        }
    }

    // A trailing newline is a line terminator, not a trailing blank line.
    while lines.last().is_some_and(|line| line.is_empty()) {
        lines.pop();
    }

    let mut out = restore_placeholders(&lines.join("\n"), &protected);
    out = collapse_blank_lines(&out);
    out
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

fn fence_length(trimmed_line: &str) -> Option<usize> {
    if let Some(rest) = trimmed_line.strip_prefix("```") {
        return Some(3 + rest.chars().take_while(|ch| *ch == '`').count());
    }
    if let Some(rest) = trimmed_line.strip_prefix("~~~") {
        return Some(3 + rest.chars().take_while(|ch| *ch == '~').count());
    }
    None
}

fn is_horizontal_rule(trimmed_line: &str) -> bool {
    let compact: String = trimmed_line.chars().filter(|ch| !ch.is_whitespace()).collect();
    if compact.len() < 3 {
        return false;
    }
    let first = compact.chars().next().expect("len checked");
    matches!(first, '-' | '*' | '_') && compact.chars().all(|ch| ch == first)
}

fn remove_heading(line: &str) -> String {
    let trimmed = line.trim_start();
    let hashes = trimmed.chars().take_while(|ch| *ch == '#').count();
    if hashes == 0 || hashes > 6 {
        return line.to_string();
    }
    let after_hashes = &trimmed[hashes..];
    if !after_hashes.starts_with(' ') && !after_hashes.is_empty() {
        return line.to_string();
    }
    let mut text = after_hashes.trim().to_string();
    // Remove optional closing hashes (`## Header ##` -> `Header`).
    while text.ends_with('#') {
        text.pop();
    }
    text.trim().to_string()
}

fn remove_blockquote(line: &str) -> String {
    let mut current = line.to_string();
    loop {
        let trimmed = current.trim_start();
        let Some(rest) = trimmed.strip_prefix('>') else {
            break;
        };
        current = match rest.strip_prefix(' ') {
            Some(rest) => rest.to_string(),
            None => rest.to_string(),
        };
    }
    current
}

fn remove_list_marker(line: &str) -> String {
    let trimmed = line.trim_start();
    for marker in ["- ", "* ", "+ "] {
        if let Some(rest) = trimmed.strip_prefix(marker) {
            return rest.to_string();
        }
    }

    let mut digit_count = 0usize;
    for ch in trimmed.chars() {
        if ch.is_ascii_digit() {
            digit_count += 1;
        } else {
            break;
        }
    }
    if digit_count == 0 {
        return line.to_string();
    }
    let after_digits = &trimmed[digit_count..];
    if let Some(rest) = after_digits
        .strip_prefix(". ")
        .or_else(|| after_digits.strip_prefix(") "))
    {
        return rest.to_string();
    }
    line.to_string()
}

fn process_inline(line: &str, protected: &mut Vec<String>) -> String {
    let line = protect_code_spans(line, protected);
    let line = convert_autolinks(&line, protected);
    let line = convert_images_and_links(&line, protected);
    let line = remove_strikethrough(&line);
    remove_emphasis(&line)
}

fn make_placeholder(protected: &mut Vec<String>, text: String) -> String {
    let index = protected.len();
    protected.push(text);
    format!("{PLACEHOLDER_PREFIX}{index}{PLACEHOLDER_PREFIX}")
}

fn restore_placeholders(text: &str, protected: &[String]) -> String {
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    loop {
        let Some(start) = rest.find(PLACEHOLDER_PREFIX) else {
            out.push_str(rest);
            break;
        };
        out.push_str(&rest[..start]);
        let after_start = &rest[start + PLACEHOLDER_PREFIX.len_utf8()..];
        let Some(end_rel) = after_start.find(PLACEHOLDER_PREFIX) else {
            out.push_str(after_start);
            break;
        };
        let index_text = &after_start[..end_rel];
        if let Ok(index) = index_text.parse::<usize>() {
            if let Some(value) = protected.get(index) {
                out.push_str(value);
            }
        }
        rest = &after_start[end_rel + PLACEHOLDER_PREFIX.len_utf8()..];
    }
    out
}

fn protect_code_spans(line: &str, protected: &mut Vec<String>) -> String {
    let mut out = String::new();
    let mut rest = line;
    loop {
        let Some(start) = rest.find('`') else {
            out.push_str(rest);
            break;
        };
        out.push_str(&rest[..start]);
        let after_start = &rest[start + 1..];
        let Some(end_rel) = after_start.find('`') else {
            out.push_str(after_start);
            break;
        };
        let code = &after_start[..end_rel];
        out.push_str(&make_placeholder(protected, code.to_string()));
        rest = &after_start[end_rel + 1..];
    }
    out
}

fn is_url(target: &str) -> bool {
    target.starts_with("http://")
        || target.starts_with("https://")
        || target.starts_with("ftp://")
        || target.starts_with("mailto:")
        || target.starts_with("//")
        || target.starts_with("www.")
}

fn convert_autolinks(line: &str, protected: &mut Vec<String>) -> String {
    let mut out = String::new();
    let mut rest = line;
    loop {
        let Some(start) = rest.find('<') else {
            out.push_str(rest);
            break;
        };
        out.push_str(&rest[..start]);
        let after_start = &rest[start + 1..];
        let Some(end_rel) = after_start.find('>') else {
            out.push('<');
            out.push_str(after_start);
            break;
        };
        let inner = &after_start[..end_rel];
        if is_url(inner) {
            out.push_str(&make_placeholder(protected, inner.to_string()));
        } else {
            out.push('<');
            out.push_str(inner);
            out.push('>');
        }
        rest = &after_start[end_rel + 1..];
    }
    out
}

fn convert_images_and_links(line: &str, protected: &mut Vec<String>) -> String {
    let mut out = String::new();
    let mut rest = line;
    loop {
        let image_start = rest.find("![");
        let link_start = rest.find('[');
        let start = match (image_start, link_start) {
            (Some(a), Some(b)) => Some(a.min(b)),
            (Some(a), None) => Some(a),
            (None, Some(b)) => Some(b),
            (None, None) => None,
        };
        let Some(start) = start else {
            out.push_str(rest);
            break;
        };
        out.push_str(&rest[..start]);
        let after = &rest[start..];
        let is_image = after.starts_with("![");
        let text_prefix_len = if is_image { 2 } else { 1 };

        let Some(close_bracket_rel) = after[text_prefix_len..].find(']') else {
            out.push_str(after);
            break;
        };
        let text_end = text_prefix_len + close_bracket_rel;
        let text = &after[text_prefix_len..text_end];
        let after_bracket = &after[text_end + 1..];

        if !after_bracket.starts_with('(') {
            out.push_str(after);
            break;
        }

        let Some(close_paren_rel) = after_bracket[1..].find(')') else {
            out.push_str(after);
            break;
        };
        let target_raw = &after_bracket[1..1 + close_paren_rel];
        let target = target_raw.split_whitespace().next().unwrap_or("").trim();

        if is_image {
            out.push_str(text);
        } else if target.starts_with('#') || target.is_empty() {
            out.push_str(text);
        } else {
            out.push_str(text);
            out.push_str(" (");
            out.push_str(&make_placeholder(protected, target.to_string()));
            out.push(')');
        }

        rest = &after_bracket[1 + close_paren_rel + 1..];
    }
    out
}

fn remove_strikethrough(line: &str) -> String {
    remove_paired_markers(line, "~~")
}

fn remove_paired_markers(line: &str, marker: &str) -> String {
    let mut out = String::new();
    let mut rest = line;
    loop {
        let Some(start) = rest.find(marker) else {
            out.push_str(rest);
            break;
        };
        out.push_str(&rest[..start]);
        let after_start = &rest[start + marker.len()..];
        let Some(end_rel) = after_start.find(marker) else {
            out.push_str(after_start);
            break;
        };
        out.push_str(&after_start[..end_rel]);
        rest = &after_start[end_rel + marker.len()..];
    }
    out
}

fn remove_emphasis(line: &str) -> String {
    let line = remove_paired_markers(line, "***");
    let line = remove_paired_markers(&line, "___");
    let line = remove_paired_markers(&line, "**");
    let line = remove_paired_markers(&line, "__");
    let line = remove_paired_markers(&line, "*");
    remove_paired_markers(&line, "_")
}

fn collapse_blank_lines(text: &str) -> String {
    let mut result: Vec<&str> = Vec::new();
    let mut blank_run = 0usize;
    for line in text.split('\n') {
        if line.is_empty() {
            blank_run += 1;
            if blank_run <= 2 {
                result.push(line);
            }
        } else {
            blank_run = 0;
            result.push(line);
        }
    }
    result.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn removes_heading_markers() {
        assert_eq!(markdown_to_text("# Header"), "Header");
        assert_eq!(markdown_to_text("## Sub heading ##"), "Sub heading");
        assert_eq!(markdown_to_text("####### Not a heading"), "####### Not a heading");
    }

    #[test]
    fn removes_emphasis() {
        assert_eq!(markdown_to_text("**bold** and *italic*"), "bold and italic");
        assert_eq!(markdown_to_text("__bold__ and _italic_"), "bold and italic");
        assert_eq!(markdown_to_text("***both***"), "both");
    }

    #[test]
    fn removes_strikethrough_and_code_spans() {
        assert_eq!(markdown_to_text("~~gone~~ and `code`"), "gone and code");
        assert_eq!(markdown_to_text("`a * b`"), "a * b");
    }

    #[test]
    fn removes_list_markers_and_blockquotes() {
        assert_eq!(markdown_to_text("- item one"), "item one");
        assert_eq!(markdown_to_text("1. first"), "first");
        assert_eq!(markdown_to_text("2) second"), "second");
        assert_eq!(markdown_to_text("> quoted"), "quoted");
    }

    #[test]
    fn removes_horizontal_rules() {
        assert_eq!(markdown_to_text("before\n---\nafter"), "before\nafter");
        assert_eq!(markdown_to_text("before\n***\nafter"), "before\nafter");
        assert_eq!(markdown_to_text("before\n___\nafter"), "before\nafter");
    }

    #[test]
    fn link_handling() {
        assert_eq!(
            markdown_to_text("[site](https://example.com)"),
            "site (https://example.com)"
        );
        assert_eq!(markdown_to_text("[section](#part)"), "section");
        assert_eq!(markdown_to_text("[file](other-file.md)"), "file (other-file.md)");
        assert_eq!(markdown_to_text("[empty]()"), "empty");
    }

    #[test]
    fn image_keeps_alt_text() {
        assert_eq!(markdown_to_text("![alt text](img.png)"), "alt text");
    }

    #[test]
    fn autolinks_become_plain_url() {
        assert_eq!(
            markdown_to_text("<https://example.com/a_b>"),
            "https://example.com/a_b"
        );
    }

    #[test]
    fn fenced_code_blocks_are_kept_verbatim() {
        let md = "before\n```\nlet x = 1;\n*not italic*\n```\nafter\n";
        assert_eq!(markdown_to_text(md), "before\nlet x = 1;\n*not italic*\nafter");
    }

    #[test]
    fn crlf_is_normalized() {
        assert_eq!(markdown_to_text("# H\r\n\r\ntext"), "H\n\ntext");
    }
}
