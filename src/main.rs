use std::env;

use anyhow::Result;
use bourso_api::client::trade::order::OrderSide;
use clap::{Arg, Command, builder::ValueParser};

use validate::validate_account_id;

mod settings;
mod validate;

#[tokio::main]
async fn main() -> Result<()> {
    const VERSION: Option<&str> = option_env!("CARGO_PKG_VERSION");

    env::set_var("RUST_LOG", "info");
    pretty_env_logger::init();

    let account_arg = Arg::new("account")
        .short('a')
        .long("account")
        .help(
            r#"The account to use by its 'id' (e.g: 'e51f635524a7d506e4d4a7a8088b6278').
    You can get this info with the command `bourso accounts`"#
        )
        .default_value("PEA")
        .value_parser(clap::value_parser!(String)) // Enforce input as String
        .value_parser(ValueParser::new(validate_account_id))
        .required(true);

    let matches = Command::new("bourso")
        .version(VERSION.unwrap_or("0.0.1"))
        .author("@azerpas")
        .about("BoursoBank/Boursorama CLI")
        .subcommand_required(true)
        .arg_required_else_help(true)
        .subcommand(
            Command::new("accounts")
                .about("Manage your saving accounts")
                .arg(
                    Arg::new("bank")
                        .long("banking")
                        .help("List all your base banking accounts")
                )
                .arg(
                    Arg::new("saving")
                        .long("saving")
                        .help("List all your saving accounts")
                )
                .arg(
                    Arg::new("trading")
                        .long("trading")
                        .help("List all your trading accounts")
                )
                .arg(
                    Arg::new("loans")
                        .long("loans")
                        .help("List all your loans")
                )
        )
        .subcommand(
            Command::new("config")
                .about("Configure BoursoBank/Boursorama CLI")
                .arg(
                    Arg::new("username")
                        .short('u')
                        .long("username")
                        .help("Your customer id")
                        .required(true)
                )
        )
        // .subcommand( // interactive mode
        .subcommand(
            Command::new("trade")
                .about("Trade with your accounts")
                .subcommand(
                    Command::new("order")
                    .subcommand(
                        Command::new("list")
                            .about("List all your current orders")
                            .arg(account_arg.clone())
                    )
                    .subcommand(
                        Command::new("new")
                            .about("Place a new order")
                            .arg(
                                Arg::new("side")
                                .long("side")
                                .help("The side of the order (buy/sell)")
                                .required(true)
                                .value_parser(clap::value_parser!(OrderSide))
                            )
                            .arg(account_arg.clone())
                            .arg(
                                Arg::new("symbol")
                                .long("symbol")
                                .help("The symbol id of the order (e.g: '1rTCW8')")
                                .required(true)
                            )
                            .arg(
                                Arg::new("quantity")
                                .short('q')
                                .long("quantity")
                                .help("The quantity of the order (e.g: '1')")
                                .required(true)
                                .value_parser(clap::value_parser!(usize))
                            )
                            // Price limit
                            // Validity date
                            // TODO: handle other types of orders
                    )
                    .subcommand(
                        Command::new("cancel")
                            .about("Cancel an order")
                            .arg(account_arg.clone())
                    )
                    .subcommand_required(true)
                )
                .subcommand_required(true)
        )
        .get_matches();

    bourso_cli::parse_matches(matches).await?;

    Ok(())
}


