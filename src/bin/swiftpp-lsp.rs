use tower_lsp::{LspService, Server};
use swiftpp::lsp::SwiftPPLanguageServer;

#[tokio::main]
async fn main() {
    let stdin = tokio::io::stdin();
    let stdout = tokio::io::stdout();

    let (service, socket) = LspService::new(|client| SwiftPPLanguageServer::new(client));
    Server::new(stdin, stdout, socket).serve(service).await;
}
