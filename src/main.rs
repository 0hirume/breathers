//! Blank-line spacing for source code.

mod command;
mod compilation;
mod configuration;
mod languages;
mod server;
mod spacing;
mod syntax;

use std::{
    env,
    io::{self, IsTerminal as _},
    process::ExitCode,
};

fn main() -> ExitCode {
    if !io::stderr().is_terminal() || env::var_os("NO_COLOR").is_some() {
        console::set_colors_enabled_stderr(false);
    }

    match command::run() {
        Ok(status) => status,

        Err(error) => {
            command::report(&error);

            ExitCode::FAILURE
        }
    }
}
