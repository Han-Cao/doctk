//! CLI feature: PDF/AI page-size and color-mode checker.

use std::path::PathBuf;

use clap::{Arg, ArgAction, ArgMatches, Command};
use doctk_core::features::pdf_checker;
use doctk_core::units::{pt_to_mm, Unit};
use doctk_core::{DoctkError, Result};

pub fn cli() -> Command {
    Command::new("pdf")
        .about("PDF/AI page-size and color-mode checks")
        .subcommand_required(true)
        .arg_required_else_help(true)
        .subcommand(
            Command::new("check")
                .about("Check whether PDF/AI pages fit a paper size")
                .arg(
                    Arg::new("FILES")
                        .value_name("FILES")
                        .required(true)
                        .num_args(1..)
                        .help("PDF or AI files to inspect"),
                )
                .arg(
                    Arg::new("paper")
                        .long("paper")
                        .value_name("PRESET")
                        .default_value("a4")
                        .help("Paper-size preset: a4, a3, letter, legal, tabloid"),
                )
                .arg(
                    Arg::new("width")
                        .long("width")
                        .value_name("VALUE")
                        .help("Custom paper width (requires --height)"),
                )
                .arg(
                    Arg::new("height")
                        .long("height")
                        .value_name("VALUE")
                        .help("Custom paper height (requires --width)"),
                )
                .arg(
                    Arg::new("unit")
                        .long("unit")
                        .value_name("UNIT")
                        .default_value("mm")
                        .help("Unit for custom width/height: mm, cm, in, pt"),
                )
                .arg(
                    Arg::new("tolerance")
                        .long("tolerance")
                        .value_name("VALUE")
                        .default_value("0.5")
                        .help("Fit tolerance in the tolerance unit"),
                )
                .arg(
                    Arg::new("tolerance-unit")
                        .long("tolerance-unit")
                        .value_name("UNIT")
                        .default_value("mm")
                        .help("Unit for tolerance: mm, cm, in, pt"),
                )
                .arg(
                    Arg::new("ignore-orientation")
                        .long("ignore-orientation")
                        .action(ArgAction::SetTrue)
                        .help("Allow landscape pages to fit a portrait paper size"),
                )
                .arg(
                    Arg::new("json")
                        .long("json")
                        .action(ArgAction::SetTrue)
                        .help("Emit a JSON report"),
                )
                .arg(
                    Arg::new("list")
                        .long("list")
                        .action(ArgAction::SetTrue)
                        .help("Emit one row per page instead of a per-file summary"),
                ),
        )
}

pub fn run(matches: &ArgMatches) -> Result<i32> {
    let Some(("check", sub)) = matches.subcommand() else {
        return Err(DoctkError::InvalidArgument(
            "expected `check` subcommand".to_string(),
        ));
    };

    let files: Vec<String> = sub
        .get_many::<String>("FILES")
        .expect("clap requires FILES")
        .cloned()
        .collect();

    let preset = sub.get_one::<String>("paper").map(String::as_str);
    let width = sub
        .get_one::<String>("width")
        .map(|s| parse_f64(s, "--width"))
        .transpose()?;
    let height = sub
        .get_one::<String>("height")
        .map(|s| parse_f64(s, "--height"))
        .transpose()?;
    let unit = sub.get_one::<String>("unit").map(String::as_str);

    if width.is_some() != height.is_some() {
        return Err(DoctkError::InvalidArgument(
            "both --width and --height must be provided for a custom size".to_string(),
        ));
    }

    let tolerance_value = parse_f64(
        sub.get_one::<String>("tolerance")
            .map(String::as_str)
            .unwrap_or("0.5"),
        "--tolerance",
    )?;
    let tolerance_unit = Unit::parse(
        sub.get_one::<String>("tolerance-unit")
            .map(String::as_str)
            .unwrap_or("mm"),
    )
    .ok_or_else(|| DoctkError::InvalidArgument("invalid --tolerance-unit".to_string()))?;
    let tolerance_pt = tolerance_unit.to_points(tolerance_value);

    let paper = pdf_checker::resolve_paper(preset, width, height, unit)?;

    let paths: Vec<PathBuf> = files.iter().map(PathBuf::from).collect();
    let reports = pdf_checker::inspect_pdfs(
        &paths,
        &paper,
        tolerance_pt,
        sub.get_flag("ignore-orientation"),
    );

    if sub.get_flag("json") {
        let json = serde_json::to_string_pretty(&reports).map_err(|err| {
            DoctkError::InvalidArgument(format!("failed to serialize JSON: {err}"))
        })?;
        println!("{json}");
    } else if sub.get_flag("list") {
        print_page_list(&reports);
    } else {
        print_summary(&reports);
    }

    let has_error = reports.iter().any(|r| r.error.is_some());
    let has_fail = reports.iter().any(|r| r.pages.iter().any(|p| !p.fits));

    if has_error {
        Ok(1)
    } else if has_fail {
        Ok(3)
    } else {
        Ok(0)
    }
}

fn parse_f64(s: &str, flag: &str) -> Result<f64> {
    s.parse::<f64>()
        .map_err(|_| DoctkError::InvalidArgument(format!("invalid numeric value `{s}` for {flag}")))
}

fn fmt_mm(pt: f64) -> String {
    format!("{:.2}", pt_to_mm(pt))
}

fn color_mode_name(mode: pdf_checker::ColorMode) -> &'static str {
    mode.as_str()
}

fn print_summary(reports: &[pdf_checker::PdfFileReport]) {
    println!(
        "{:<40} {:>5} {:>12} {:>12} {:<12} {}",
        "file", "pages", "min (mm)", "max (mm)", "color", "fit"
    );
    println!(
        "{:-<40} {:-<5} {:-<12} {:-<12} {:-<12} {:-<6}",
        "", "", "", "", "", ""
    );
    for report in reports {
        if let Some(error) = &report.error {
            println!("{:<40}  ERROR: {}", report.path, error);
            continue;
        }
        let page_count = report.pages.len();
        if page_count == 0 {
            println!("{:<40} {:>5}  no pages", report.path, page_count);
            continue;
        }
        let min_w = report
            .pages
            .iter()
            .map(|p| p.width_pt)
            .fold(f64::INFINITY, f64::min);
        let max_w = report.pages.iter().map(|p| p.width_pt).fold(0.0, f64::max);
        let min_h = report
            .pages
            .iter()
            .map(|p| p.height_pt)
            .fold(f64::INFINITY, f64::min);
        let max_h = report.pages.iter().map(|p| p.height_pt).fold(0.0, f64::max);
        let modes = report
            .pages
            .iter()
            .map(|p| color_mode_name(p.color_mode))
            .collect::<std::collections::BTreeSet<_>>();
        let failed = report.pages.iter().filter(|p| !p.fits).count();
        let fit = if failed == 0 {
            "PASS".to_string()
        } else {
            format!("{failed} FAIL")
        };
        println!(
            "{:<40} {:>5} {:>12} {:>12} {:<12} {}",
            report.path,
            page_count,
            format!("{:.2} x {:.2}", fmt_mm(min_w), fmt_mm(min_h)),
            format!("{:.2} x {:.2}", fmt_mm(max_w), fmt_mm(max_h)),
            modes.into_iter().collect::<Vec<_>>().join(","),
            fit,
        );
    }
}

fn print_page_list(reports: &[pdf_checker::PdfFileReport]) {
    println!(
        "{:<40} {:>5} {:>12} {:>12} {:<10} {:<6} {}",
        "file", "page", "width (mm)", "height (mm)", "color", "fit", "reason"
    );
    for report in reports {
        if let Some(error) = &report.error {
            println!("{:<40}  ERROR: {}", report.path, error);
            continue;
        }
        for page in &report.pages {
            println!(
                "{:<40} {:>5} {:>12} {:>12} {:<10} {:<6} {}",
                report.path,
                page.page_number,
                fmt_mm(page.width_pt),
                fmt_mm(page.height_pt),
                color_mode_name(page.color_mode),
                if page.fits { "PASS" } else { "FAIL" },
                page.fit_reason.as_deref().unwrap_or(""),
            );
        }
    }
}
