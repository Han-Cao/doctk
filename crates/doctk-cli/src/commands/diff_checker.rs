//! CLI feature: diff checker.

use std::io::IsTerminal;

use clap::{Arg, ArgAction, ArgMatches, Command};
use doctk_core::features::diff_checker::{
    self, LineKind, SideBySideDiff, TrackChangeKind, TrackChangesDiff, WordRange,
};
use doctk_core::{DoctkError, Result};

use super::{read_input, write_output};

pub fn cli() -> Command {
    Command::new("diff")
        .about("Visualize the diff between two text files")
        .arg(
            Arg::new("LEFT")
                .value_name("LEFT")
                .required(true)
                .help("Left/old file path, or `-` for stdin"),
        )
        .arg(
            Arg::new("RIGHT")
                .value_name("RIGHT")
                .required(true)
                .help("Right/new file path, or `-` for stdin"),
        )
        .arg(
            Arg::new("format")
                .long("format")
                .value_name("FORMAT")
                .value_parser(["side-by-side", "track-changes", "unified"])
                .default_value("side-by-side")
                .help("Output format"),
        )
        .arg(
            Arg::new("output")
                .short('o')
                .long("output")
                .value_name("OUTPUT")
                .default_value("-")
                .help("Output file, or `-` for stdout"),
        )
        .arg(
            Arg::new("exit-code")
                .long("exit-code")
                .action(ArgAction::SetTrue)
                .help("Exit with code 1 when the inputs differ"),
        )
}

pub fn run(matches: &ArgMatches) -> Result<i32> {
    let left_path = matches
        .get_one::<String>("LEFT")
        .map(String::as_str)
        .unwrap();
    let right_path = matches
        .get_one::<String>("RIGHT")
        .map(String::as_str)
        .unwrap();
    let format = matches
        .get_one::<String>("format")
        .map(String::as_str)
        .unwrap_or("side-by-side");
    let output_path = matches
        .get_one::<String>("output")
        .map(String::as_str)
        .unwrap_or("-");
    let exit_code = matches.get_flag("exit-code");

    let left = read_input(left_path)?;
    let right = read_input(right_path)?;
    diff_checker::validate_input_size(&left, &right)?;

    let rendered = match format {
        "side-by-side" => render_side_by_side(
            &diff_checker::diff_side_by_side(&left, &right),
            std::io::stdout().is_terminal() && output_path == "-",
        ),
        "track-changes" => {
            render_track_changes_html(&diff_checker::diff_track_changes(&left, &right))
        }
        "unified" => diff_checker::diff_unified(&left, &right),
        other => {
            return Err(DoctkError::InvalidArgument(format!(
                "unknown diff format `{other}`"
            )))
        }
    };
    write_output(output_path, &rendered)?;

    if exit_code && !diff_checker::is_identical(&left, &right) {
        Ok(1)
    } else {
        Ok(0)
    }
}

fn render_side_by_side(diff: &SideBySideDiff, use_color: bool) -> String {
    let mut out = String::new();
    for line in &diff.lines {
        match line.kind {
            LineKind::Equal => {
                out.push_str(&format!("  {}  |  {}\n", line.left_text, line.right_text));
            }
            LineKind::Delete => {
                out.push_str("- ");
                out.push_str(&paint(
                    &line.left_text,
                    &line.word_diff,
                    true,
                    use_color,
                    "\x1b[31m",
                ));
                out.push_str("  |\n");
            }
            LineKind::Insert => {
                out.push_str("  |  + ");
                out.push_str(&paint(
                    &line.right_text,
                    &line.word_diff,
                    false,
                    use_color,
                    "\x1b[32m",
                ));
                out.push('\n');
            }
            LineKind::Replace => {
                out.push_str("- ");
                out.push_str(&paint(
                    &line.left_text,
                    &line.word_diff,
                    true,
                    use_color,
                    "\x1b[31m",
                ));
                out.push_str("  |  + ");
                out.push_str(&paint(
                    &line.right_text,
                    &line.word_diff,
                    false,
                    use_color,
                    "\x1b[32m",
                ));
                out.push('\n');
            }
        }
    }
    out
}

fn paint(
    text: &str,
    ranges: &[WordRange],
    left_side: bool,
    use_color: bool,
    color: &str,
) -> String {
    if !use_color || ranges.is_empty() {
        return text.to_string();
    }
    let mut out = String::new();
    let mut cursor = 0usize;
    for range in ranges {
        let (start, len) = if left_side {
            (range.left_start, range.left_len)
        } else {
            (range.right_start, range.right_len)
        };
        if len == 0 {
            continue;
        }
        if start < cursor || start + len > text.len() {
            continue;
        }
        out.push_str(&text[cursor..start]);
        out.push_str(color);
        out.push_str("\x1b[7m");
        out.push_str(&text[start..start + len]);
        out.push_str("\x1b[0m");
        cursor = start + len;
    }
    out.push_str(&text[cursor..]);
    out
}

fn render_track_changes_html(diff: &TrackChangesDiff) -> String {
    let mut body = String::from("<pre>\n");
    for segment in &diff.segments {
        match segment.kind {
            TrackChangeKind::Equal => body.push_str(&escape_html(&segment.text)),
            TrackChangeKind::Deleted => {
                body.push_str("<del style=\"color:red;text-decoration:line-through\">");
                body.push_str(&escape_html(&segment.text));
                body.push_str("</del>");
            }
            TrackChangeKind::Inserted => {
                body.push_str("<ins style=\"color:green;text-decoration:underline\">");
                body.push_str(&escape_html(&segment.text));
                body.push_str("</ins>");
            }
        }
    }
    body.push_str("</pre>\n");
    format!(
        "<!doctype html>\n<html>\n<head><meta charset=\"utf-8\"><title>Track changes</title></head>\n<body>\n{body}</body>\n</html>\n"
    )
}

fn escape_html(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}
