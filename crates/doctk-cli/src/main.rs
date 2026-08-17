use std::process::ExitCode;

mod commands;

fn main() -> ExitCode {
    let matches = commands::build_cli().get_matches();
    match commands::dispatch(&matches) {
        Ok(code) => ExitCode::from(code as u8),
        Err(err) => {
            eprintln!("error: {}", err.to_user_message());
            ExitCode::from(err.exit_code() as u8)
        }
    }
}
