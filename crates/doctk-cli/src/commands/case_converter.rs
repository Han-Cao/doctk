//! CLI feature: case conversion.

use clap::{Arg, ArgAction, ArgMatches, Command};
use doctk_core::features::case_converter::{self, CaseMode};
use doctk_core::{DoctkError, Result};

use super::{read_input, write_output};

const MODES: [&str; 5] = ["sentence", "lower", "upper", "capitalized", "title"];

pub fn cli() -> Command {
    Command::new("case")
        .about("Convert text between sentence, lower, upper, capitalized, and title case")
        .arg(
            Arg::new("mode")
                .value_name("MODE")
                .required(true)
                .value_parser(MODES)
                .help("Conversion mode: sentence, lower, upper, capitalized, or title"),
        )
        .arg(
            Arg::new("input")
                .value_name("INPUT")
                .default_value("-")
                .help("Input file, or `-` for stdin"),
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
            Arg::new("proper_noun")
                .long("proper-noun")
                .value_name("NOUN")
                .action(ArgAction::Append)
                .help("Proper noun or phrase for sentence/title case; repeat for multiple entries"),
        )
}

pub fn run(matches: &ArgMatches) -> Result<i32> {
    let mode = match matches
        .get_one::<String>("mode")
        .map(String::as_str)
        .unwrap_or_default()
    {
        "sentence" => CaseMode::Sentence,
        "lower" => CaseMode::Lower,
        "upper" => CaseMode::Upper,
        "capitalized" => CaseMode::Capitalized,
        "title" => CaseMode::Title,
        other => {
            return Err(DoctkError::InvalidArgument(format!(
                "unknown case mode `{other}`"
            )))
        }
    };

    let proper_nouns: Vec<String> = matches
        .get_many::<String>("proper_noun")
        .map(|values| values.cloned().collect())
        .unwrap_or_default();

    if !matches!(mode, CaseMode::Sentence | CaseMode::Title) && !proper_nouns.is_empty() {
        return Err(DoctkError::InvalidArgument(
            "--proper-noun can only be used with sentence or title mode".to_string(),
        ));
    }

    let input = read_input(
        matches
            .get_one::<String>("input")
            .map(String::as_str)
            .unwrap_or("-"),
    )?;
    let output = case_converter::convert(&input, mode, &proper_nouns);

    write_output(
        matches
            .get_one::<String>("output")
            .map(String::as_str)
            .unwrap_or("-"),
        &output,
    )?;

    Ok(0)
}
