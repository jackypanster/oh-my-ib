//! `omi` — read-only Interactive Brokers CLI. Each invocation: parse → load+merge
//! config → connect → request → emit. JSON to stdout, error envelope to stderr.

use clap::error::ErrorKind as ClapErrorKind;
use clap::Parser;

use oh_my_ib::cli::{Cli, Command, Format};
use oh_my_ib::config::Config;
use oh_my_ib::error::AppError;
use oh_my_ib::{ib, output};

fn main() {
    let cli = match Cli::try_parse() {
        Ok(cli) => cli,
        Err(err) => match err.kind() {
            // Explicit --help / --version are not failures: clap renders them (to stdout), exit 0.
            ClapErrorKind::DisplayHelp | ClapErrorKind::DisplayVersion => {
                let _ = err.print();
                std::process::exit(0);
            }
            // A bare `omi` (or any missing required subcommand) is an INVALID call, not a success:
            // emit the JSON usage envelope + non-zero exit so an agent/script never reads it as ok.
            ClapErrorKind::MissingSubcommand
            | ClapErrorKind::DisplayHelpOnMissingArgumentOrSubcommand => {
                let app_err = AppError::usage(
                    "a subcommand is required",
                    "run `omi --help` to list commands",
                );
                output::emit_error(&app_err);
                std::process::exit(app_err.exit_code());
            }
            // Any other parse failure becomes the structured JSON error envelope.
            _ => {
                let msg = err
                    .to_string()
                    .lines()
                    .next()
                    .unwrap_or("invalid arguments")
                    .trim_start_matches("error: ")
                    .trim()
                    .to_string();
                let app_err = AppError::usage(msg, "command-line arguments");
                output::emit_error(&app_err);
                std::process::exit(app_err.exit_code());
            }
        },
    };
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
        Command::Brief => ib::brief(&config),
        Command::Account => ib::account(&config),
        Command::Pnl => ib::pnl(&config),
        Command::PnlByPosition => ib::pnl_by_position(&config),
        Command::Positions => ib::positions(&config),
        Command::Orders => ib::orders(&config),
        Command::Executions => ib::executions(&config),
        Command::Quote(args) => ib::quote(&config, args),
        Command::Contract(args) => ib::contract(&config, args),
        Command::History(args) => ib::history(&config, args),
        Command::Search(args) => ib::search(&config, args),
        Command::CompletedOrders => ib::completed_orders(&config),
        Command::Buy(args) => ib::buy(&config, args),
        Command::Sell(args) => ib::sell(&config, args),
        Command::Cancel(args) => ib::cancel(&config, args),
        Command::OptionChain(args) => ib::option_chain(&config, args),
        Command::OptionQuote(args) => ib::option_quote(&config, args),
        Command::OptionBuy(args) => ib::option_buy(&config, args),
        Command::OptionSell(args) => ib::option_sell(&config, args),
        Command::OptionCombo(args) => ib::option_combo(&config, args),
        Command::OptionClose(args) => ib::option_close(&config, args),
        Command::SmaSignal(args) => ib::sma_signal_cmd(&config, args),
        Command::GridTick(args) => ib::grid_tick(&config, args),
        Command::SmaTick(args) => ib::sma_tick_cmd(&config, args),
    }
}
