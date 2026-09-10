use std::{
    collections::BTreeMap,
    future::{Future, ready},
    path::PathBuf,
    sync::Mutex,
};

use tower_lsp_server::{
    LanguageServer, LspService, Server,
    jsonrpc::{Error, Result},
    ls_types::{
        DidChangeTextDocumentParams, DidCloseTextDocumentParams, DidOpenTextDocumentParams,
        DocumentFormattingParams, InitializeParams, InitializeResult, OneOf, Position, Range,
        ServerCapabilities, TextDocumentSyncCapability, TextDocumentSyncKind, TextEdit, Uri,
    },
};

use crate::{
    Language,
    configuration::{Configuration, Selection},
};

struct Document {
    language: Option<Language>,
    version: i32,
    text: Option<String>,
}

struct Backend {
    documents: Mutex<BTreeMap<Uri, Document>>,
    directory: PathBuf,
    configuration: Option<PathBuf>,
}

fn language(identifier: &str, uri: &Uri) -> Option<Language> {
    match identifier {
        "rust" => Some(Language::Rust),
        "lua" => Some(Language::Lua),
        "luau" => Some(Language::Luau),
        "c" => Some(Language::C),
        "cpp" | "c++" => Some(Language::CPlusPlus),
        "python" => Some(Language::Python),
        "javascript" | "javascriptreact" | "jsx" => Some(Language::Javascript),
        "typescript" => Some(Language::Typescript),
        "typescriptreact" | "tsx" => Some(Language::Tsx),

        _ if uri.scheme().as_str().eq_ignore_ascii_case("file") => {
            uri.to_file_path().and_then(|path| Language::infer(&path))
        }

        _ => None,
    }
}

fn end(source: &str) -> Result<Position> {
    let mut position = Position::default();
    let mut characters = source.chars().peekable();

    while let Some(character) = characters.next() {
        if matches!(character, '\r' | '\n') {
            if character == '\r' && characters.peek() == Some(&'\n') {
                characters.next();
            }

            position.line = position
                .line
                .checked_add(1)
                .ok_or_else(|| Error::invalid_params("Document has too many lines"))?;

            position.character = 0;
        } else {
            position.character = position
                .character
                .checked_add(u32::try_from(character.len_utf16()).unwrap())
                .ok_or_else(|| Error::invalid_params("Document line is too long"))?;
        }
    }

    Ok(position)
}

impl LanguageServer for Backend {
    fn initialize(&self, _: InitializeParams) -> impl Future<Output = Result<InitializeResult>> {
        ready(Ok(InitializeResult {
            capabilities: ServerCapabilities {
                text_document_sync: Some(TextDocumentSyncCapability::Kind(
                    TextDocumentSyncKind::FULL,
                )),
                document_formatting_provider: Some(OneOf::Left(true)),
                ..ServerCapabilities::default()
            },
            ..InitializeResult::default()
        }))
    }

    fn shutdown(&self) -> impl Future<Output = Result<()>> {
        ready(Ok(()))
    }

    fn did_open(&self, parameters: DidOpenTextDocumentParams) -> impl Future<Output = ()> {
        let document = parameters.text_document;

        self.documents.lock().unwrap().insert(
            document.uri.clone(),
            Document {
                language: language(&document.language_id, &document.uri),
                version: document.version,
                text: Some(document.text),
            },
        );

        ready(())
    }

    fn did_change(&self, parameters: DidChangeTextDocumentParams) -> impl Future<Output = ()> {
        let mut documents = self.documents.lock().unwrap();

        if let Some(document) = documents.get_mut(&parameters.text_document.uri)
            && parameters.text_document.version > document.version
        {
            document.version = parameters.text_document.version;

            for change in parameters.content_changes {
                document.text = if change.range.is_none() {
                    Some(change.text)
                } else {
                    None
                };
            }
        }

        ready(())
    }

    fn did_close(&self, parameters: DidCloseTextDocumentParams) -> impl Future<Output = ()> {
        self.documents
            .lock()
            .unwrap()
            .remove(&parameters.text_document.uri);

        ready(())
    }

    fn formatting(
        &self,
        parameters: DocumentFormattingParams,
    ) -> impl Future<Output = Result<Option<Vec<TextEdit>>>> {
        ready(self.format(parameters))
    }
}

impl Backend {
    fn format(&self, parameters: DocumentFormattingParams) -> Result<Option<Vec<TextEdit>>> {
        let documents = self.documents.lock().unwrap();
        let uri = parameters.text_document.uri;

        let document = documents
            .get(&uri)
            .ok_or_else(|| Error::invalid_params("Document is not open"))?;

        let language = document
            .language
            .ok_or_else(|| Error::invalid_params("Unsupported document language"))?;

        let source = document
            .text
            .as_deref()
            .ok_or_else(|| Error::invalid_params("Full document synchronization is required"))?;

        let path = if uri.scheme().as_str().eq_ignore_ascii_case("file") {
            Some(
                uri.to_file_path()
                    .ok_or_else(|| Error::invalid_params("Invalid file URI"))?,
            )
        } else {
            None
        };

        let directory = path
            .as_ref()
            .and_then(|path| path.parent())
            .unwrap_or(&self.directory);

        let (configuration, root) =
            Configuration::load_in(directory, self.configuration.as_deref())
                .map_err(Error::invalid_params)?;

        let selection = Selection::new(&configuration, root).map_err(Error::invalid_params)?;

        if let Some(path) = path.as_ref() {
            let directory = std::fs::canonicalize(directory)
                .map_err(|error| Error::invalid_params(error.to_string()))?;

            let name = path
                .file_name()
                .ok_or_else(|| Error::invalid_params("Document URI has no file name"))?;

            if !selection.includes(&directory.join(name)) {
                return Ok(None);
            }
        }

        let formatted = language
            .breathe(source, &configuration)
            .map_err(Error::invalid_params)?;

        if formatted == source {
            return Ok(None);
        }

        Ok(Some(vec![TextEdit::new(
            Range::new(Position::default(), end(source)?),
            formatted,
        )]))
    }
}

pub fn run(configuration: Option<PathBuf>) -> std::result::Result<(), String> {
    let directory = std::env::current_dir().map_err(|error| error.to_string())?;
    let configuration = configuration.map(|path| directory.join(path));

    tokio::runtime::Builder::new_current_thread()
        .build()
        .map_err(|error| error.to_string())?
        .block_on(async {
            let (service, socket) = LspService::new(|_| Backend {
                documents: Mutex::default(),
                directory,
                configuration,
            });

            Server::new(tokio::io::stdin(), tokio::io::stdout(), socket)
                .concurrency_level(1)
                .serve(service)
                .await;
        });

    Ok(())
}

#[cfg(test)]
mod tests {
    #[test]
    fn counts_utf16_positions_and_line_endings() {
        for (source, line, character) in [
            ("", 0, 0),
            ("🙂", 0, 2),
            ("a\r\n🙂", 1, 2),
            ("a\rb\n", 2, 0),
        ] {
            assert_eq!(
                super::end(source).unwrap(),
                tower_lsp_server::ls_types::Position::new(line, character)
            );
        }
    }
}
