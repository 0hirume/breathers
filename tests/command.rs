//! Command-line behavior and protocol contracts.

#[path = "command/batch.rs"]
mod batch;

#[path = "command/compilation.rs"]
mod compilation;

#[path = "command/input.rs"]
mod input;

#[path = "command/languages.rs"]
mod languages;

#[path = "command/selection.rs"]
mod selection;

#[path = "command/server.rs"]
mod server;

#[path = "support/mod.rs"]
mod support;
