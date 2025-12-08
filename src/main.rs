mod arxiv_search;
mod cdp;
mod config;
mod models;

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
    /// Configuration management
    Config {
        #[command(subcommand)]
        command: ConfigCommands,
    },
}

#[derive(Subcommand)]
enum ConfigCommands {
    /// Set a config value
    Set {
        key: String,
        value: String,
    },
    /// Get a config value
    Get {
        key: String,
    },
    /// List all config values
    List,
    /// Show config file path
    Path,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    let mut config = Config::load()?;

    if let Commands::Config { command } = &cli.command {
        match command {
            ConfigCommands::Set { key, value } => {
                config.set(key, value)?;
                config.save()?;
                println!("Config updated: {} = {}", key, value);
            }
            ConfigCommands::Get { key } => {
                let value = config.get(key)?;
                println!("{}", value);
            }
            ConfigCommands::List => {
                let json = serde_json::to_string_pretty(&config)?;
                println!("{}", json);
            }
            ConfigCommands::Path => {
                let path = Config::config_path()?;
                println!("{}", path.display());
            }
        }
        return Ok(());
    }

    if cli.head {
        config.headless = false;
    }

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
        Commands::Config { .. } => unreachable!(),
    }

    Ok(())
}
