mod error;
mod tools;

use rmcp::{ServiceExt, transport::stdio};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let server = tools::D3skServer::new();
    let service = server.serve(stdio()).await?;
    service.waiting().await?;
    Ok(())
}
