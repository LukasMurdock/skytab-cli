use clap::{Args, Parser, Subcommand, ValueEnum};

#[derive(Debug, Clone, Parser)]
#[command(name = "skytab")]
#[command(about = "Rust CLI for SkyTab API")]
pub struct Cli {
    #[arg(long, global = true)]
    pub json: bool,

    #[arg(short = 'v', long = "verbose", action = clap::ArgAction::Count, global = true)]
    pub verbose: u8,

    #[arg(long, global = true)]
    pub base_url: Option<String>,

    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Debug, Clone, Subcommand)]
pub enum Commands {
    Auth(AuthArgs),
    Locations(LocationsArgs),
    Accounts(AccountsArgs),
    Reports(ReportsArgs),
    Timeclock(TimeclockArgs),
    Payments(PaymentsArgs),
    Request(RequestArgs),
    Doctor,
}

#[derive(Debug, Clone, Args)]
pub struct AuthArgs {
    #[command(subcommand)]
    pub command: AuthSubcommand,
}

#[derive(Debug, Clone, Subcommand)]
pub enum AuthSubcommand {
    Login,
    SetCredentials {
        #[arg(long)]
        username: String,
        #[arg(long)]
        password: Option<String>,
        #[arg(long, default_value_t = false)]
        prompt_password: bool,
        #[arg(long)]
        base_url: Option<String>,
    },
}

#[derive(Debug, Clone, Args)]
pub struct LocationsArgs {
    #[command(subcommand)]
    pub command: LocationsSubcommand,
}

#[derive(Debug, Clone, Subcommand)]
pub enum LocationsSubcommand {
    List,
    SetDefault {
        #[arg(long)]
        location_id: i64,
    },
    ShowDefault,
    ClearDefault,
}

#[derive(Debug, Clone, Args)]
pub struct AccountsArgs {
    #[command(subcommand)]
    pub command: AccountsSubcommand,
}

#[derive(Debug, Clone, Subcommand)]
pub enum AccountsSubcommand {
    Preferences {
        #[arg(long)]
        account_id: String,
    },
}

#[derive(Debug, Clone, Args)]
pub struct ReportsArgs {
    #[command(subcommand)]
    pub command: ReportsSubcommand,
}

#[derive(Debug, Clone, Subcommand)]
pub enum ReportsSubcommand {
    ActivitySummary {
        #[arg(long)]
        start: String,
        #[arg(long)]
        end: String,
        #[arg(long)]
        location: Option<i64>,
    },
    DiscountSummary {
        #[arg(long)]
        start: String,
        #[arg(long)]
        end: String,
        #[arg(long)]
        location: Vec<i64>,
    },
    HourlySales {
        #[arg(long)]
        start: String,
        #[arg(long)]
        end: String,
        #[arg(long)]
        location: Vec<i64>,
    },
    TicketDetailClosed {
        #[arg(long)]
        start: String,
        #[arg(long)]
        end: String,
        #[arg(long)]
        location: Vec<i64>,
    },
    SalesSummaryByItem {
        #[arg(long)]
        start: String,
        #[arg(long)]
        end: String,
        #[arg(long)]
        location: Vec<i64>,
    },
    SalesSummaryByRevenueClass {
        #[arg(long)]
        start: String,
        #[arg(long)]
        end: String,
        #[arg(long)]
        location: Vec<i64>,
    },
    TillTransaction {
        #[arg(long)]
        start: String,
        #[arg(long)]
        end: String,
        #[arg(long)]
        location: Vec<i64>,
    },
    Payroll {
        #[arg(long)]
        start: String,
        #[arg(long)]
        end: String,
        #[arg(long)]
        location: Vec<i64>,
    },
}

#[derive(Debug, Clone, Args)]
pub struct TimeclockArgs {
    #[command(subcommand)]
    pub command: TimeclockSubcommand,
}

#[derive(Debug, Clone, Subcommand)]
pub enum TimeclockSubcommand {
    Shifts {
        #[arg(long)]
        location_id: Option<i64>,
        #[arg(long)]
        start: String,
        #[arg(long)]
        end: String,
        #[arg(long, default_value = "clockedInAt asc")]
        order: String,
        #[arg(long, default_value_t = 100)]
        limit: usize,
    },
}

#[derive(Debug, Clone, Args)]
pub struct PaymentsArgs {
    #[command(subcommand)]
    pub command: PaymentsSubcommand,
}

#[derive(Debug, Clone, Subcommand)]
pub enum PaymentsSubcommand {
    Transactions {
        #[arg(long)]
        start: String,
        #[arg(long)]
        end: String,
        #[arg(long)]
        location: Vec<i64>,
        #[arg(long)]
        order_type: Option<String>,
    },
}

#[derive(Debug, Clone, ValueEnum)]
pub enum HttpMethod {
    Get,
    Post,
    Put,
    Patch,
    Delete,
}

#[derive(Debug, Clone, Args)]
pub struct RequestArgs {
    #[arg(long, value_enum)]
    pub method: HttpMethod,
    #[arg(long)]
    pub path: String,
    #[arg(long)]
    pub query: Vec<String>,
    #[arg(long)]
    pub body: Option<String>,
}
