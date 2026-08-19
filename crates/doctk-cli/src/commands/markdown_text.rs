//! CLI feature: markdown -> plain text.

use clap::{Arg, ArgMatches, Command};
use doctk_core::features::markdown_text;
use doctk_core::Result;

use super::{read_input, write_output};

pub fn cli() -> Command {
    Command::new("md2text")
        .about("Convert markdown to plain text")
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
}

pub fn run(matches: &ArgMatches) -> Result<i32> {
    let input = read_input(matches.get_one::<String>("input").map(String::as_str).unwrap_or("-"))?;
    let output = markdown_text::markdown_to_text(&input);
    write_output(
        matches.get_one::<String>("output").map(String::as_str).unwrap_or("-"),
        &output,
    )?;
    Ok(0)
}
