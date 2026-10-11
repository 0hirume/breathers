use std::{
    io::{self, Write as _},
    process::ExitCode,
};

use clap::Parser as _;

use crate::{configuration, server};

use super::{arguments::Arguments, batch::batch};

pub(crate) fn run() -> Result<ExitCode, String> {
    let arguments = Arguments::parse();

    if arguments.server {
        return server::run(arguments.config).map(|()| ExitCode::SUCCESS);
    }

    if arguments.schema {
        println!(
            "{}",
            serde_json::to_string_pretty(&schemars::schema_for!(configuration::Configuration))
                .map_err(|error| error.to_string())?
        );

        return Ok(ExitCode::SUCCESS);
    }

    let input_language = if arguments.paths.iter().any(|path| path.as_os_str() == "-") {
        if arguments.paths.len() != 1 {
            return Err("Stdin (-) must be the only input path".into());
        }

        Some(arguments.language.ok_or("Stdin requires --language (-l)")?)
    } else {
        None
    };

    let (configuration, root) = configuration::load(arguments.config.as_deref())?;
    let selection = configuration::Selection::new(&configuration, root)?;

    if let Some(language) = input_language {
        let source =
            io::read_to_string(io::stdin().lock()).map_err(|error| format!("stdin: {error}"))?;

        let formatted = language
            .format(&source, &configuration, None)
            .map_err(|error| format!("stdin: {error}"))?;

        let mut output = io::stdout().lock();

        return output
            .write_all(formatted.as_bytes())
            .and_then(|()| output.flush())
            .map(|()| ExitCode::SUCCESS)
            .map_err(|error| format!("stdout: {error}"));
    }

    batch(arguments, &configuration, &selection)
}
