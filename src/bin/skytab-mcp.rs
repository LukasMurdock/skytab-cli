use clap::Parser;

use skytab_cli::logging::init_tracing;
use skytab_cli::mcp_server;

#[derive(Debug, Clone, Parser)]
#[command(name = "skytab-mcp")]
#[command(about = "Read-only MCP server for SkyTab API")]
struct McpCli {
    #[arg(short = 'v', long = "verbose", action = clap::ArgAction::Count)]
    verbose: u8,

    #[arg(long)]
    base_url: Option<String>,
}

#[tokio::main]
async fn main() {
    let cli = McpCli::parse();
    init_tracing(cli.verbose, true);

    if let Err(err) = mcp_server::serve_stdio(cli.base_url).await {
        eprintln!("Error: {err}");
        std::process::exit(1);
    }
}
