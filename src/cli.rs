use bourso_api::client::trade::order::OrderSide;
use clap::{Args, Parser, Subcommand};

// TODO: add debug option
// TODO: add value parser to validate account ID

#[derive(Parser)]
#[command(version, author, about, long_about = None)]
pub struct Cli {
    /// Optional path to credentials JSON file
    #[arg(short, long, value_name = "FILE")]
    pub credentials: Option<String>,

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
    /// Your customer ID
    #[arg(short, long, value_name = "ID")]
    pub username: String,
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
    /// The account to use by its ID (32 hex chars) (e.g: "e51f635524a7d506e4d4a7a8088b6278")
    ///
    /// You can get your account ID with the `bourso accounts` command
    #[arg(short, long, value_name = "ID")]
    pub account: String,

    /// The side of the order (buy/sell)
    #[arg(long, value_parser = clap::value_parser!(OrderSide))]
    pub side: OrderSide,

    /// The symbol ID of the order (e.g: "1rTCW8")
    #[arg(long, value_name = "ID")]
    pub symbol: String,

    /// The quantity of the order (e.g: 1)
    #[arg(short, long)]
    pub quantity: usize,
}

#[derive(Args)]
pub struct OrderCancelArgs {}

#[derive(Args)]
pub struct QuoteArgs {
    /// The symbol ID of the stock (e.g: "1rTCW8")
    #[arg(long, value_name = "ID")]
    pub symbol: String,

    /// The length period of the stock (1, 5, 30, 90, 180, 365, 1825, 3650)
    #[arg(
        long,
        default_value = "30",
        value_parser = ["1","5","30","90","180","365","1825","3650"]
    )]
    pub length: String,

    /// The interval of the stock (use "0" for default)
    #[arg(long, default_value = "0", value_parser = ["0"])]
    pub interval: String,

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

    /// Get the last value of the stock, sets `length=0` and `interval=0`
    Last,
}

#[derive(Args)]
pub struct TransferArgs {
    /// The source account ID (32 hex chars), you can get it with the `bourso accounts` command
    #[arg(long = "from", value_name = "ID")]
    pub from_account: String,

    /// The destination account ID (32 hex chars), you can getit with the `bourso accounts` command
    #[arg(long = "to", value_name = "ID")]
    pub to_account: String,

    /// The amount to transfer
    #[arg(long)]
    pub amount: String,

    /// The reason for the transfer (max 50 chars)
    #[arg(long)]
    pub reason: Option<String>,
}
