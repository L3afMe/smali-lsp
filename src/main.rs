#![feature(impl_trait_in_bindings)]

pub mod server;

use std::collections::HashMap;

use lspower::{jsonrpc::Result as LspResult, lsp::*, Client, LanguageServer, LspService, Server};
use serde_json::Value;
use server::{helper::lsp_range_to_range, validation::validate};
use tokio::sync::RwLock;

#[derive(Debug)]
struct Document {
    pub uri:     Url,
    pub content: RwLock<String>,
}

impl Document {
    async fn update(&self, range: Range, content: String) {
        let range = lsp_range_to_range(range, &self.content.read().await);
        self.content.write().await.replace_range(range, &content);
    }
}

#[derive(Debug)]
struct DocumentCache {
    pub map: RwLock<HashMap<Url, Document>>,
}

impl DocumentCache {
    async fn update(&self, params: &DidChangeTextDocumentParams) -> Result<(), String> {
        for change in &params.content_changes {
            let lock = self.map.read().await;
            let doc = lock.get(&params.text_document.uri);

            if doc.is_none() {
                return Err("Unable to get document to update".to_string());
            }

            if change.range.is_none() {
                return Err("Unable to get range to update".to_string());
            }

            let doc = doc.unwrap();
            let range = change.range.unwrap();

            doc.update(range, change.text.clone()).await;
        }

        Ok(())
    }

    async fn did_open(&self, params: &DidOpenTextDocumentParams) {
        if !{ self.map.read().await.contains_key(&params.text_document.uri) } {
            self.map
                .write()
                .await
                .insert(params.text_document.uri.clone(), Document {
                    uri:     params.text_document.uri.clone(),
                    content: RwLock::new(params.text_document.text.clone()),
                });
        }
    }

    async fn did_close(&self, params: &DidCloseTextDocumentParams) {
        if !self.map.read().await.contains_key(&params.text_document.uri) {
            self.map.write().await.remove(&params.text_document.uri.clone());
        }
    }
}

#[derive(Debug)]
struct Backend {
    client:    Client,
    documents: DocumentCache,
}

impl Backend {
    async fn validate(&self, uri: Url) {
        let file_name = {
            let uri = uri.to_string();
            if uri.contains('/') { uri.split('/').last().unwrap().to_string() } else { uri }
        }
        .replace("%24", "$")
        .replace("%20", " ");
        self.client.log_message(MessageType::Info, format!("[validator] Validating {}", &file_name),) .await;

        if self.documents.map.read().await.contains_key(&uri) {
            let content = {
                let lock = self.documents.map.read().await;
                let doc = lock.get(&uri).unwrap();

                let lock = doc.content.read().await;
                lock.clone()
            };

            match validate(content) {
                Ok(diags) => {
                    self.client.publish_diagnostics(uri, diags, None).await;
                    self.client.log_message(MessageType::Info, format!("[validator] Succesfully validated {}", &file_name),) .await;
                },
                Err(why) => {
                    self.client.show_message(MessageType::Error, why.clone()).await;
                    self.client.log_message(MessageType::Info, format!("[validator] Error while validating {}", &file_name)).await;
                    self.client.log_message(MessageType::Info, format!("[validator] {}", why)).await;
                },
            }

            return;
        }

        self.client .show_message(MessageType::Error, "Unable to get current document for validation") .await;
        self.client.log_message(MessageType::Info, "[validator] Unable to get current document for validation.").await;
        self.client.log_message(MessageType::Info, format!("[validator] Uri: {}", &file_name)).await;
    }
}

#[lspower::async_trait]
impl LanguageServer for Backend {
    async fn initialize(&self, _: InitializeParams) -> LspResult<InitializeResult> {
        Ok(InitializeResult {
            server_info:  None,
            capabilities: ServerCapabilities {
                text_document_sync: Some(TextDocumentSyncCapability::Kind(TextDocumentSyncKind::Incremental)),
                completion_provider: Some(CompletionOptions {
                    resolve_provider: Some(false),
                    trigger_characters: Some(
                        // Do these actually change anything??
                        vec![".".to_string(), "L".to_string(), "v".to_string(), "p".to_string()],
                    ),
                    ..Default::default()
                }),
                execute_command_provider: Some(ExecuteCommandOptions {
                    commands: vec!["smali-lsp.format".to_string()],
                    ..Default::default()
                }),
                workspace: Some(WorkspaceServerCapabilities {
                    workspace_folders: Some(WorkspaceFoldersServerCapabilities {
                        supported:            Some(true),
                        change_notifications: Some(OneOf::Left(true)),
                    }),
                    ..Default::default()
                }),
                ..ServerCapabilities::default()
            },
        })
    }

    async fn initialized(&self, _: InitializedParams) {
        self.client
            .show_message(MessageType::Info, "Initialized smali-lsp")
            .await;
    }

    async fn shutdown(&self) -> LspResult<()> {
        Ok(())
    }

    async fn did_change_workspace_folders(&self, _: DidChangeWorkspaceFoldersParams) {
    }

    async fn did_change_configuration(&self, _: DidChangeConfigurationParams) {
    }

    async fn did_change_watched_files(&self, _: DidChangeWatchedFilesParams) {
    }

    async fn execute_command(&self, _: ExecuteCommandParams) -> LspResult<Option<Value>> {
        match self
            .client
            .apply_edit(WorkspaceEdit::default(), Default::default())
            .await
        {
            Ok(res) if res.applied => self.client.log_message(MessageType::Info, "applied").await,
            Ok(_) => self.client.log_message(MessageType::Info, "rejected").await,
            Err(err) => self.client.log_message(MessageType::Error, err).await,
        }

        Ok(None)
    }

    async fn did_open(&self, params: DidOpenTextDocumentParams) {
        self.documents.did_open(&params).await;

        self.validate(params.text_document.uri).await;
    }

    async fn did_close(&self, params: DidCloseTextDocumentParams) {
        self.documents.did_close(&params).await;

        self.validate(params.text_document.uri).await;
    }

    async fn did_change(&self, params: DidChangeTextDocumentParams) {
        if let Err(why) = self.documents.update(&params).await {
            self.client.show_message(MessageType::Error, why).await;
        }

        self.validate(params.text_document.uri).await;
    }

    async fn did_save(&self, _: DidSaveTextDocumentParams) {
        self.client.log_message(MessageType::Info, "file saved!").await;
    }

    async fn completion(&self, _: CompletionParams) -> LspResult<Option<CompletionResponse>> {
        Ok(Some(CompletionResponse::Array(vec![
            CompletionItem::new_simple("Hello".to_string(), "Some detail".to_string()),
            CompletionItem::new_simple("Bye".to_string(), "More detail".to_string()),
        ])))
    }
}

#[tokio::main]
async fn main() {
    let stdin = tokio::io::stdin();
    let stdout = tokio::io::stdout();

    let (service, messages) = LspService::new(|client| Backend {
        client,
        documents: DocumentCache {
            map: RwLock::new(HashMap::new()),
        },
    });
    Server::new(stdin, stdout).interleave(messages).serve(service).await;
}
