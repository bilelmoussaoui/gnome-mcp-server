mod mcp;
mod resources;
mod tools;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_ansi(false)
        .with_max_level(tracing::Level::INFO)
        .init();

    mcp::Server::run().await
}
