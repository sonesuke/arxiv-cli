use crate::core::{ArxivClient, Config};
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "arxiv-cli")]
#[command(about = "Search and fetch papers from Arxiv", long_about = None)]
#[command(version)]
pub struct Cli {
    /// Show browser window (disable headless mode)
    #[arg(long)]
    pub head: bool,

    /// Verbose output (show debug information)
    #[arg(long)]
    pub verbose: bool,

    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
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

        /// Filter by category (e.g., cs.AI, physics.cond-mat, math.NA)
        #[arg(long)]
        category: Option<String>,
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
    /// Start MCP server
    Mcp,
}

#[derive(Subcommand)]
pub enum ConfigCommands {
    /// Set a config value
    Set { key: String, value: String },
    /// Get a config value
    Get { key: String },
    /// List all config values
    List,
    /// Show config file path
    Path,
}

pub async fn run() -> anyhow::Result<()> {
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

    if let Commands::Mcp = cli.command {
        // This will be handled in main.rs calling mcp::run, but we need to pass the config
        return crate::mcp::run(config).await;
    }

    let client = ArxivClient::new(&config).await?;

    match cli.command {
        Commands::Search { query, limit, after, before, category } => {
            let papers = client.search(&query, limit, after, before, category, cli.verbose).await?;
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
        _ => unreachable!(),
    }

    Ok(())
}
