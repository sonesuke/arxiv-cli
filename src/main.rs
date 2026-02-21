mod cli;
mod core;
mod mcp;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    cli::run().await
}
