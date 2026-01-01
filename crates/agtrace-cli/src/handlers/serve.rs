use crate::mcp;
use agtrace_sdk::Client;
use anyhow::Result;

pub async fn handle(client: &Client) -> Result<()> {
    // Clone the client since run_server takes ownership
    mcp::run_server((*client).clone()).await
}
