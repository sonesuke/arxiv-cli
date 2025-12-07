pub mod arxiv_search;
pub mod cdp;
pub mod config;
pub mod models;

use arxiv_search::ArxivClient;
use clap::{Parser, Subcommand};
use config::Config;

#[derive(Parser)]
#[command(name = "arxiv-cli")]
#[command(about = "Search and fetch papers from Arxiv", long_about = None)]
struct Cli {
    /// Show browser window (disable headless mode)
    #[arg(long)]
    head: bool,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Search for papers
    Search {
        /// Search query
        #[arg(short, long)]
        query: String,

        /// Limit the number of results
        #[arg(short, long)]
        limit: Option<usize>,

        /// Filter by date (after), YYYY-MM-DD
        #[arg(long)]
        after: Option<String>,

        /// Filter by date (before), YYYY-MM-DD
        #[arg(long)]
        before: Option<String>,
    },
    /// Fetch paper details by ID
    Fetch {
        /// Arxiv ID
        id: String,

        /// Output raw HTML
        #[arg(long)]
        raw: bool,
    },
}

pub async fn run() -> anyhow::Result<()> {
    let cli = Cli::parse();

    let config = Config { headless: !cli.head };

    let client = ArxivClient::new(&config).await?;

    match cli.command {
        Commands::Search { query, limit, after, before } => {
            let papers = client.search(&query, limit, after, before).await?;
            let json = serde_json::to_string_pretty(&papers)?;
            println!("{}", json);
        }
        Commands::Fetch { id, raw } => {
            if raw {
                let bytes = client.fetch_pdf(&id).await?;
                use std::io::Write;
                std::io::stdout().write_all(&bytes)?;
            } else {
                let paper = client.fetch(&id).await?;
                let json = serde_json::to_string_pretty(&paper)?;
                println!("{}", json);
            }
        }
    }

    Ok(())
}
