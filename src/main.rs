#[tokio::main]
async fn main() -> anyhow::Result<()> {
    arxiv_cli::cli::run().await
}
