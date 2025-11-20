use clap::{value_parser, Args, Parser, Subcommand};
use std::path::PathBuf;

use bourso_api::types::{
    AccountId, ClientNumber, MoneyAmount, OrderQuantity, OrderSide, QuoteLength, QuotePeriod,
    SymbolId, TransferReason,
};

// TODO: add debug option
// TODO: add type to fix primitive obsession and value_parser (AccountId, QuoteInterval, QuoteLength, ...)

#[derive(Parser)]
#[command(version, author, about, long_about = None)]
pub struct Cli {
    /// Optional path to credentials JSON file
    #[arg(short, long, value_name = "FILE", value_parser = value_parser!(PathBuf))]
    pub credentials: Option<PathBuf>,

    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Configure the CLI
    Config(ConfigArgs),

    /// List your accounts
    Accounts(AccountsArgs),

    /// Trade with your accounts
    Trade(TradeArgs),

    /// Get quotes details for a given symbol over a time period (do not require authentication)
    Quote(QuoteArgs),

    /// Transfer funds between your accounts
    Transfer(TransferArgs),
}

#[derive(Args)]
pub struct ConfigArgs {
    /// Your client number
    #[arg(short, long, value_name = "ID", value_parser = value_parser!(ClientNumber))]
    pub client_number: ClientNumber,
}

#[derive(Args)]
pub struct AccountsArgs {
    /// List all your base banking accounts
    #[arg(long)]
    pub banking: bool,

    /// List all your saving accounts
    #[arg(long)]
    pub saving: bool,

    /// List all your trading accounts
    #[arg(long)]
    pub trading: bool,

    /// List all your loans
    #[arg(long)]
    pub loans: bool,
}

#[derive(Args)]
pub struct TradeArgs {
    #[command(subcommand)]
    pub command: TradeCommands,
}

#[derive(Subcommand)]
pub enum TradeCommands {
    /// Trade orders
    Order(OrderArgs),
}

#[derive(Args)]
pub struct OrderArgs {
    #[command(subcommand)]
    pub command: OrderSubcommands,
}

#[derive(Subcommand)]
pub enum OrderSubcommands {
    /// List your current orders (coming soon)
    List(OrderListArgs),

    /// Place a new order
    New(OrderNewArgs),

    /// Cancel an order (coming soon)
    Cancel(OrderCancelArgs),
}

#[derive(Args)]
pub struct OrderListArgs {}

#[derive(Args)]
pub struct OrderNewArgs {
    /// Account to use by its ID (32 hex chars), you can get it with the `bourso-cli accounts` command
    #[arg(short, long, value_name = "ID", value_parser = value_parser!(AccountId))]
    pub account: AccountId,

    /// Side of the order
    #[arg(long, default_value = "buy")]
    pub side: OrderSide,

    /// Symbol ID of the order (e.g: "1rTCW8")
    #[arg(long, value_name = "ID", value_parser = value_parser!(SymbolId))]
    pub symbol: SymbolId,

    /// Quantity of the order (e.g: 1)
    #[arg(short, long, value_parser = value_parser!(OrderQuantity))]
    pub quantity: OrderQuantity,
}

#[derive(Args)]
pub struct OrderCancelArgs {}

#[derive(Args)]
pub struct QuoteArgs {
    /// Symbol ID of the stock (e.g: "1rTCW8")
    #[arg(long, value_name = "ID", value_parser = value_parser!(SymbolId))]
    pub symbol: SymbolId,

    /// Length period of the stock
    #[arg(long, default_value = "30")]
    pub length: QuoteLength,

    /// Period of the stock
    #[arg(long, default_value = "0", value_parser = value_parser!(QuotePeriod))]
    pub period: QuotePeriod,

    #[command(subcommand)]
    pub view: Option<QuoteView>,
}

#[derive(Subcommand)]
pub enum QuoteView {
    /// Get the highest value of the stock for the given length and interval
    Highest,

    /// Get the lowest value of the stock for the given length and interval
    Lowest,

    /// Get the average value of the stock for the given length and interval
    Average,

    /// Get the volume of the stock for the given length and interval
    Volume,

    /// Get the last value of the stock, sets `length=1` and `interval=0`
    Last,
}

#[derive(Args)]
pub struct TransferArgs {
    /// Source account ID (32 hex chars), you can get it with the `bourso-cli accounts` command
    #[arg(long = "from", value_name = "ID", value_parser = value_parser!(AccountId))]
    pub from_account: AccountId,

    /// Destination account ID (32 hex chars), you can get it with the `bourso-cli accounts` command
    #[arg(long = "to", value_name = "ID", value_parser = value_parser!(AccountId))]
    pub to_account: AccountId,

    /// Amount to transfer
    #[arg(long, value_parser = value_parser!(MoneyAmount))]
    pub amount: MoneyAmount,

    /// Reason for the transfer (max 50 chars)
    #[arg(long, value_parser = value_parser!(TransferReason))]
    pub reason: Option<TransferReason>,
}
