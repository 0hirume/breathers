//! Blank-line spacing for source code.

mod command;
mod configuration;
mod error;
mod languages;
mod spacing;
mod syntax;

use std::process::ExitCode;

fn main() -> ExitCode {
    command::run()
}
