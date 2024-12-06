use tower_lsp::jsonrpc::Result;
use tower_lsp::lsp_types::*;
use tower_lsp::{Client, LanguageServer, LspService, Server};
use std::sync::Arc;
use tokio::sync::Mutex;

use crate::compiler::{lexer, parser, analyzer};

pub struct SwiftPPLanguageServer {
    client: Client,
    document_map: Arc<Mutex<DocumentMap>>,
}

struct DocumentMap {
    documents: std::collections::HashMap<Url, String>,
}

#[tower_lsp::async_trait]
impl LanguageServer for SwiftPPLanguageServer {
    async fn initialize(&self, _: InitializeParams) -> Result<InitializeResult> {
        Ok(InitializeResult {
            capabilities: ServerCapabilities {
                text_document_sync: Some(TextDocumentSyncCapability::Kind(
                    TextDocumentSyncKind::INCREMENTAL,
                )),
                completion_provider: Some(CompletionOptions {
                    resolve_provider: Some(false),
                    trigger_characters: Some(vec![".".to_string()]),
                    work_done_progress_options: Default::default(),
                    all_commit_characters: None,
                }),
                definition_provider: Some(OneOf::Left(true)),
                hover_provider: Some(HoverProviderCapability::Simple(true)),
                diagnostic_provider: Some(DiagnosticServerCapabilities::Options(
                    DiagnosticOptions {
                        identifier: None,
                        inter_file_dependencies: true,
                        workspace_diagnostics: true,
                        work_done_progress_options: WorkDoneProgressOptions {
                            work_done_progress: None,
                        },
                    },
                )),
                ..ServerCapabilities::default()
            },
            server_info: None,
        })
    }

    async fn initialized(&self, _: InitializedParams) {
        self.client
            .log_message(MessageType::INFO, "Swift++ Language Server initialized!")
            .await;
    }

    async fn shutdown(&self) -> Result<()> {
        Ok(())
    }

    async fn did_open(&self, params: DidOpenTextDocumentParams) {
        let mut documents = self.document_map.lock().await;
        documents
            .documents
            .insert(params.text_document.uri, params.text_document.text);
        
        self.validate_document(&params.text_document.uri, &params.text_document.text)
            .await;
    }

    async fn did_change(&self, params: DidChangeTextDocumentParams) {
        let mut documents = self.document_map.lock().await;
        if let Some(content) = documents.documents.get_mut(&params.text_document.uri) {
            for change in params.content_changes {
                if let Some(range) = change.range {
                    // Apply incremental changes
                    let start_pos = self.position_to_index(content, range.start);
                    let end_pos = self.position_to_index(content, range.end);
                    content.replace_range(start_pos..end_pos, &change.text);
                } else {
                    // Full document update
                    *content = change.text;
                }
            }
            
            self.validate_document(&params.text_document.uri, content).await;
        }
    }

    async fn completion(&self, params: CompletionParams) -> Result<Option<CompletionResponse>> {
        let documents = self.document_map.lock().await;
        if let Some(content) = documents.documents.get(&params.text_document_position.text_document.uri) {
            // Provide context-aware completions
            let items = self.get_completion_items(content, params.text_document_position.position).await;
            Ok(Some(CompletionResponse::Array(items)))
        } else {
            Ok(None)
        }
    }

    async fn goto_definition(
        &self,
        params: GotoDefinitionParams,
    ) -> Result<Option<GotoDefinitionResponse>> {
        let documents = self.document_map.lock().await;
        if let Some(content) = documents.documents.get(&params.text_document_position_params.text_document.uri) {
            // Find symbol definition
            let location = self.find_definition(content, params.text_document_position_params.position).await;
            Ok(location.map(GotoDefinitionResponse::Scalar))
        } else {
            Ok(None)
        }
    }
}

impl SwiftPPLanguageServer {
    pub fn new(client: Client) -> Self {
        SwiftPPLanguageServer {
            client,
            document_map: Arc::new(Mutex::new(DocumentMap {
                documents: std::collections::HashMap::new(),
            })),
        }
    }

    async fn validate_document(&self, uri: &Url, content: &str) {
        // Perform lexical and syntactic analysis
        let mut lexer = lexer::Lexer::new(content);
        let tokens: Vec<_> = std::iter::from_fn(|| {
            let token = lexer.next_token();
            if token.token_type == lexer::TokenType::EOF {
                None
            } else {
                Some(token)
            }
        })
        .collect();

        let mut diagnostics = Vec::new();

        // Parse and collect syntax errors
        let mut parser = parser::Parser::new(tokens);
        match parser.parse() {
            Ok(ast) => {
                // Perform semantic analysis
                let mut analyzer = analyzer::SemanticAnalyzer::new();
                if let Err(errors) = analyzer.analyze(&ast) {
                    for error in errors {
                        diagnostics.push(Diagnostic {
                            range: Range {
                                start: Position::new(0, 0), // TODO: Add proper error positions
                                end: Position::new(0, 0),
                            },
                            severity: Some(DiagnosticSeverity::ERROR),
                            code: None,
                            code_description: None,
                            source: Some("swiftpp".to_string()),
                            message: error,
                            related_information: None,
                            tags: None,
                            data: None,
                        });
                    }
                }
            }
            Err(error) => {
                diagnostics.push(Diagnostic {
                    range: Range {
                        start: Position::new(0, 0),
                        end: Position::new(0, 0),
                    },
                    severity: Some(DiagnosticSeverity::ERROR),
                    code: None,
                    code_description: None,
                    source: Some("swiftpp".to_string()),
                    message: error,
                    related_information: None,
                    tags: None,
                    data: None,
                });
            }
        }

        self.client
            .publish_diagnostics(uri.clone(), diagnostics, None)
            .await;
    }

    fn position_to_index(&self, content: &str, position: Position) -> usize {
        let mut current_line = 0;
        let mut current_character = 0;
        let mut index = 0;

        for (i, c) in content.chars().enumerate() {
            if current_line == position.line as usize
                && current_character == position.character as usize
            {
                return i;
            }

            if c == '\n' {
                current_line += 1;
                current_character = 0;
            } else {
                current_character += 1;
            }
            index = i;
        }

        index + 1
    }

    async fn get_completion_items(&self, content: &str, position: Position) -> Vec<CompletionItem> {
        // TODO: Implement context-aware completion
        vec![
            CompletionItem {
                label: "fn".to_string(),
                kind: Some(CompletionItemKind::KEYWORD),
                detail: Some("Function declaration".to_string()),
                ..Default::default()
            },
            CompletionItem {
                label: "let".to_string(),
                kind: Some(CompletionItemKind::KEYWORD),
                detail: Some("Variable declaration".to_string()),
                ..Default::default()
            },
            // Add more completion items
        ]
    }

    async fn find_definition(&self, content: &str, position: Position) -> Option<Location> {
        // TODO: Implement symbol definition lookup
        None
    }
}
