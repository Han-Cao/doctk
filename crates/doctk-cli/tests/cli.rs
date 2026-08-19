//! End-to-end CLI tests.

use std::path::PathBuf;
use std::process::Command;

fn doctk() -> Command {
    Command::new(env!("CARGO_BIN_EXE_doctk"))
}

fn fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name)
}

#[test]
fn table_md2tsv_from_file() {
    let input = fixture("table.md");
    std::fs::write(&input, "| a | b |\n|---|---|\n| 1 | 2 |\n").unwrap();
    let output = doctk()
        .args(["table", "md2tsv", input.to_str().unwrap()])
        .output()
        .unwrap();
    assert!(output.status.success());
    assert_eq!(String::from_utf8(output.stdout).unwrap(), "a\tb\n1\t2");
}

#[test]
fn table_tsv2md_from_stdin() {
    let mut child = doctk()
        .args(["table", "tsv2md", "-"])
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .spawn()
        .unwrap();
    use std::io::Write;
    child
        .stdin
        .as_mut()
        .unwrap()
        .write_all(b"a\tb\n1\t2\n")
        .unwrap();
    let output = child.wait_with_output().unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("| a | b |"));
    assert!(stdout.contains("| 1 | 2 |"));
}

#[test]
fn table_md2tsv_invalid_table_errors() {
    let output = doctk()
        .args(["table", "md2tsv", "-"])
        .stdin(std::process::Stdio::piped())
        .output()
        .unwrap();
    // Write to a temporary file instead to avoid needing stdin for an error case.
    drop(output);
    let input = fixture("bad-table.md");
    std::fs::write(&input, "| a |\n").unwrap();
    let output = doctk()
        .args(["table", "md2tsv", input.to_str().unwrap()])
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&output.stderr).contains("delimiter"));
}

#[test]
fn diff_side_by_side_outputs_deletions() {
    let left = fixture("left.txt");
    let right = fixture("right.txt");
    std::fs::write(&left, "a\nb\n").unwrap();
    std::fs::write(&right, "a\nc\n").unwrap();
    let output = doctk()
        .args(["diff", left.to_str().unwrap(), right.to_str().unwrap()])
        .output()
        .unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("- b"));
    assert!(stdout.contains("+ c"));
}

#[test]
fn diff_exit_code_reports_difference() {
    let left = fixture("left2.txt");
    let right = fixture("right2.txt");
    std::fs::write(&left, "same\n").unwrap();
    std::fs::write(&right, "different\n").unwrap();
    let output = doctk()
        .args([
            "diff",
            left.to_str().unwrap(),
            right.to_str().unwrap(),
            "--exit-code",
        ])
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(1));
}

#[test]
fn md2text_converts_stdin_to_stdout() {
    let mut child = doctk()
        .args(["md2text", "-"])
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .spawn()
        .unwrap();
    use std::io::Write;
    child
        .stdin
        .as_mut()
        .unwrap()
        .write_all(b"# Header\n\n[site](https://example.com) and [part](#part)\n")
        .unwrap();
    let output = child.wait_with_output().unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("Header"));
    assert!(stdout.contains("site (https://example.com)"));
    assert!(stdout.contains("part"));
    assert!(!stdout.contains("#part"));
}

#[test]
fn md2text_writes_output_file() {
    let input = fixture("sample.md");
    let output_path = fixture("sample.txt");
    std::fs::write(&input, "# Title\n\nSome **bold** text\n").unwrap();
    let output = doctk()
        .args([
            "md2text",
            input.to_str().unwrap(),
            "-o",
            output_path.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert!(output.status.success());
    let text = std::fs::read_to_string(&output_path).unwrap();
    assert!(text.contains("Title"));
    assert!(text.contains("Some bold text"));
    std::fs::remove_file(&input).unwrap();
    std::fs::remove_file(&output_path).unwrap();
}

#[test]
fn pdf_check_reports_json_and_fail_exit_code() {
    let pdf = fixture("sample.pdf");
    let output = doctk()
        .args(["pdf", "check", pdf.to_str().unwrap(), "--json"])
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(3));
    let json = String::from_utf8(output.stdout).unwrap();
    let report: serde_json::Value = serde_json::from_str(&json).unwrap();
    assert_eq!(report[0]["pages"].as_array().unwrap().len(), 2);
    assert_eq!(report[0]["pages"][0]["color_mode"], "Rgb");
    assert_eq!(report[0]["pages"][1]["color_mode"], "Cmyk");
}
