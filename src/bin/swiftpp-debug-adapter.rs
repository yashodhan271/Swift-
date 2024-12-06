use dap::prelude::*;
use swiftpp::debugger::SwiftPPDebugAdapter;
use tokio;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let stdin = tokio::io::stdin();
    let stdout = tokio::io::stdout();

    let (service, socket) = DebugService::new(|client| SwiftPPDebugAdapter::new(client));
    Server::new(stdin, stdout, socket).serve(service).await?;

    Ok(())
}
