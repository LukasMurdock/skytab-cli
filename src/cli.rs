use clap::{Args, Parser, Subcommand, ValueEnum};
use std::path::PathBuf;

#[derive(Debug, Clone, Parser)]
#[command(name = "skytab")]
#[command(about = "Rust CLI for SkyTab API")]
pub struct Cli {
    #[arg(long, global = true)]
    pub json: bool,

    #[arg(long, value_enum, global = true)]
    pub format: Option<OutputFormat>,

    #[arg(long, global = true)]
    pub output: Option<PathBuf>,

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
    Completion(CompletionArgs),
    Doctor,
}

#[derive(Debug, Clone, Args)]
pub struct CompletionArgs {
    #[arg(value_enum)]
    pub shell: CompletionShell,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum CompletionShell {
    Bash,
    Zsh,
    Fish,
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

#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum OutputFormat {
    Json,
    Csv,
    Ndjson,
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
    #[arg(long, default_value_t = false)]
    pub allow_write: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_completion_subcommand() {
        let cli = Cli::try_parse_from(["skytab", "completion", "bash"])
            .expect("completion command should parse");

        match cli.command {
            Commands::Completion(args) => {
                assert!(matches!(args.shell, CompletionShell::Bash));
            }
            other => panic!("expected completion command, got: {other:?}"),
        }
    }

    #[test]
    fn parses_global_format_and_output_flags() {
        let cli = Cli::try_parse_from([
            "skytab",
            "--format",
            "csv",
            "--output",
            "report.csv",
            "reports",
            "hourly-sales",
            "--start",
            "2026-03-01",
            "--end",
            "2026-03-01",
        ])
        .expect("format and output flags should parse");

        assert!(matches!(cli.format, Some(OutputFormat::Csv)));
        assert_eq!(
            cli.output
                .as_ref()
                .map(|value| value.to_string_lossy().to_string()),
            Some("report.csv".to_string())
        );
    }

    #[test]
    fn request_allow_write_flag_defaults_false() {
        let cli = Cli::try_parse_from([
            "skytab",
            "request",
            "--method",
            "get",
            "--path",
            "/api/v2/locations",
        ])
        .expect("request command should parse");

        match cli.command {
            Commands::Request(args) => {
                assert!(matches!(args.method, HttpMethod::Get));
                assert!(!args.allow_write);
            }
            other => panic!("expected request command, got: {other:?}"),
        }
    }

    #[test]
    fn request_allow_write_flag_parses_when_present() {
        let cli = Cli::try_parse_from([
            "skytab",
            "request",
            "--method",
            "post",
            "--path",
            "/api/v2/example",
            "--allow-write",
        ])
        .expect("request command with allow-write should parse");

        match cli.command {
            Commands::Request(args) => {
                assert!(matches!(args.method, HttpMethod::Post));
                assert!(args.allow_write);
            }
            other => panic!("expected request command, got: {other:?}"),
        }
    }
}
