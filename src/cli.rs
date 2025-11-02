use bourso_api::client::trade::order::OrderSide;
use clap::{value_parser, Args, Parser, Subcommand};

// TODO: add debug option
// TODO: add type to fix primitive obsession (AccountId w/ FromStr impl)

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
    /// Account to use by its ID (32 hex chars), you can get it with the `bourso accounts` command
    #[arg(short, long, value_name = "ID", value_parser = parse_account_id)]
    pub account: String,

    /// Side of the order (buy/sell)
    #[arg(long, value_parser = clap::value_parser!(OrderSide))]
    pub side: OrderSide,

    /// Symbol ID of the order (e.g: "1rTCW8")
    #[arg(long, value_name = "ID")]
    pub symbol: String,

    /// Quantity of the order (e.g: 1)
    #[arg(short, long, value_parser = value_parser!(u64).range(1..))]
    pub quantity: u64,
}

#[derive(Args)]
pub struct OrderCancelArgs {}

#[derive(Args)]
pub struct QuoteArgs {
    /// Symbol ID of the stock (e.g: "1rTCW8")
    #[arg(long, value_name = "ID")]
    pub symbol: String,

    /// Length period of the stock (1, 5, 30, 90, 180, 365, 1825, 3650)
    #[arg(
        long,
        default_value = "30",
        value_parser = ["1","5","30","90","180","365","1825","3650"]
    )]
    pub length: String,

    /// Interval of the stock (use "0" for default)
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

    /// Get the last value of the stock, sets `length=1` and `interval=0`
    Last,
}

#[derive(Args)]
pub struct TransferArgs {
    /// Source account ID (32 hex chars), you can get it with the `bourso accounts` command
    #[arg(long = "from", value_name = "ID", value_parser = parse_account_id)]
    pub from_account: String,

    /// Destination account ID (32 hex chars), you can get it with the `bourso accounts` command
    #[arg(long = "to", value_name = "ID", value_parser = parse_account_id)]
    pub to_account: String,

    /// Amount to transfer
    #[arg(long)]
    pub amount: String,

    /// Reason for the transfer (max 50 chars)
    #[arg(long)]
    pub reason: Option<String>,
}

fn parse_account_id(s: &str) -> Result<String, String> {
    let t = s.trim();
    if t.len() == 32 && t.chars().all(|c| c.is_ascii_hexdigit()) {
        Ok(t.to_owned())
    } else {
        Err("Account ID must be 32 hex characters (0-9, a-f)".into())
    }
}
