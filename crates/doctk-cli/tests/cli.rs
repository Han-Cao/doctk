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

#[test]
fn pdf_check_recovers_whitespace_pointer_with_correct_exit_codes() {
    let pdf = fixture("startxref-whitespace.pdf");
    let original = std::fs::read(&pdf).unwrap();
    for (paper, expected_code) in [("letter", 0), ("a4", 3)] {
        let output = doctk()
            .args([
                "pdf",
                "check",
                pdf.to_str().unwrap(),
                "--paper",
                paper,
                "--json",
            ])
            .output()
            .unwrap();
        assert_eq!(output.status.code(), Some(expected_code), "{output:?}");
        let report: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
        assert!(report[0]["error"].is_null());
        let pages = report[0]["pages"].as_array().unwrap();
        assert_eq!(pages.len(), 2);
        for page in pages {
            assert_eq!(page["width_pt"], 612.0);
            assert_eq!(page["height_pt"], 792.0);
            assert_eq!(page["fits"], paper == "letter");
        }
        assert_eq!(pages[0]["color_mode"], "Rgb");
        assert_eq!(pages[1]["color_mode"], "Cmyk");
    }
    assert_eq!(std::fs::read(&pdf).unwrap(), original);
}

#[test]
fn pdf_check_continues_after_corrupt_file() {
    let corrupt = fixture("table.md");
    let valid = fixture("startxref-whitespace.pdf");
    let output = doctk()
        .args([
            "pdf",
            "check",
            corrupt.to_str().unwrap(),
            valid.to_str().unwrap(),
            "--paper",
            "letter",
            "--json",
        ])
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(1));
    let reports: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(reports.as_array().unwrap().len(), 2);
    assert!(reports[0]["error"]
        .as_str()
        .unwrap()
        .contains("cannot load"));
    assert!(reports[0]["pages"].as_array().unwrap().is_empty());
    assert!(reports[1]["error"].is_null());
    assert_eq!(reports[1]["pages"].as_array().unwrap().len(), 2);
}

#[test]
fn case_upper_converts_stdin_to_stdout() {
    let mut child = doctk()
        .args(["case", "upper", "-"])
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .spawn()
        .unwrap();

    use std::io::Write;
    child
        .stdin
        .as_mut()
        .unwrap()
        .write_all(b"hello WORLD\n")
        .unwrap();

    let output = child.wait_with_output().unwrap();
    assert!(output.status.success());
    assert_eq!(String::from_utf8(output.stdout).unwrap(), "HELLO WORLD\n");
}

#[test]
fn case_sentence_uses_repeated_proper_nouns() {
    let mut child = doctk()
        .args([
            "case",
            "sentence",
            "-",
            "--proper-noun",
            "John",
            "--proper-noun",
            "New York",
        ])
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .spawn()
        .unwrap();

    use std::io::Write;
    child
        .stdin
        .as_mut()
        .unwrap()
        .write_all(b"john met mary in new york")
        .unwrap();

    let output = child.wait_with_output().unwrap();
    assert!(output.status.success());
    assert_eq!(
        String::from_utf8(output.stdout).unwrap(),
        "John met mary in New York"
    );
}

#[test]
fn case_title_writes_output_file() {
    let input = fixture("case-title-input.txt");
    let output_path = fixture("case-title-output.txt");
    std::fs::write(&input, "the lord of the rings\n").unwrap();

    let output = doctk()
        .args([
            "case",
            "title",
            input.to_str().unwrap(),
            "-o",
            output_path.to_str().unwrap(),
        ])
        .output()
        .unwrap();

    assert!(output.status.success());
    let text = std::fs::read_to_string(&output_path).unwrap();
    assert_eq!(text, "The Lord of the Rings\n");

    std::fs::remove_file(&input).unwrap();
    std::fs::remove_file(&output_path).unwrap();
}

#[test]
fn case_title_uses_proper_nouns() {
    let mut child = doctk()
        .args(["case", "title", "-", "--proper-noun", "NASA"])
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .spawn()
        .unwrap();

    use std::io::Write;
    child
        .stdin
        .as_mut()
        .unwrap()
        .write_all(b"nasa and the space race\n")
        .unwrap();

    let output = child.wait_with_output().unwrap();
    assert!(output.status.success());
    assert_eq!(
        String::from_utf8(output.stdout).unwrap(),
        "NASA and the Space Race\n"
    );
}

#[test]
fn case_invalid_mode_is_usage_error() {
    let output = doctk().args(["case", "nope", "-"]).output().unwrap();
    assert_eq!(output.status.code(), Some(2));
}

#[test]
fn case_proper_noun_with_unsupported_mode_errors() {
    let output = doctk()
        .args(["case", "capitalized", "-", "--proper-noun", "John"])
        .output()
        .unwrap();

    assert_eq!(output.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&output.stderr).contains("--proper-noun"));
}
