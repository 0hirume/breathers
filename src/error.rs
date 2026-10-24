use std::{io, sync::Arc};

use miette::{Diagnostic, LabeledSpan, NamedSource};
use thiserror::Error;

#[derive(Debug, Diagnostic, Error)]
#[error("{message}")]
pub(crate) struct Failure {
    message: String,

    #[source]
    cause: Option<io::Error>,

    #[source_code]
    source: Option<Arc<NamedSource<String>>>,

    #[label(collection)]
    labels: Vec<LabeledSpan>,
}

impl Failure {
    pub(crate) fn syntax(source: &str, labels: Vec<LabeledSpan>) -> Self {
        Self {
            source: Some(Arc::new(NamedSource::new("source", source.to_owned()))),
            labels,
            ..Self::from("Syntax errors; no changes written")
        }
    }

    pub(crate) fn context(mut self, name: &str) -> Self {
        self.message = format!("{name}: {}", self.message);

        if let Some(source) = self.source.take() {
            self.source = Some(Arc::new(NamedSource::new(name, source.inner().clone())));
        }

        self
    }
}

impl From<String> for Failure {
    fn from(message: String) -> Self {
        Self {
            message,
            cause: None,
            source: None,
            labels: Vec::new(),
        }
    }
}

impl From<&str> for Failure {
    fn from(message: &str) -> Self {
        Self::from(message.to_owned())
    }
}

impl From<io::Error> for Failure {
    fn from(cause: io::Error) -> Self {
        Self {
            cause: Some(cause),
            ..Self::from("Filesystem operation failed")
        }
    }
}
