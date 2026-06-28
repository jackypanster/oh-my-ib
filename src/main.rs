//! `omi` — read-only Interactive Brokers CLI. Each invocation: parse → load+merge
//! config → connect → request → emit. JSON to stdout, error envelope to stderr.

use clap::Parser;

use oh_my_ib::cli::{Cli, Command, Format};
use oh_my_ib::config::Config;
use oh_my_ib::error::AppError;
use oh_my_ib::{ib, output};

fn main() {
    let cli = Cli::parse();
    let format = cli.global.format.unwrap_or(Format::Json);
    match run(&cli) {
        Ok(value) => {
            output::emit_success(&value, format);
            std::process::exit(0);
        }
        Err(err) => {
            output::emit_error(&err);
            std::process::exit(err.exit_code());
        }
    }
}

fn run(cli: &Cli) -> Result<serde_json::Value, AppError> {
    let config = Config::load()?.merge_flags(&cli.global)?;
    match &cli.command {
        Command::Health => ib::health(&config),
        Command::Account => ib::account(&config),
        Command::Positions => ib::positions(&config),
        Command::Orders => ib::orders(&config),
        Command::Quote(args) => ib::quote(&config, args),
        Command::Contract(args) => ib::contract(&config, args),
        Command::History(args) => ib::history(&config, args),
    }
}
