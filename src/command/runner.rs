use std::{
    env, fs, io,
    path::{Path, PathBuf},
    process::ExitCode,
};

use clap::Parser as _;

use crate::{configuration, error::Failure, languages::Language};

use super::{
    arguments::Arguments,
    batch::batch,
    preview::diff,
    reporting::{Reporting, Status, colors, report, stdout},
    settings::Settings,
};

pub(crate) fn run() -> ExitCode {
    let arguments = Arguments::parse();
    let colors = colors(arguments.output.color);

    match execute(&arguments) {
        Ok(status) => status,

        Err(error) => {
            report(&error, colors).unwrap_or(());

            ExitCode::FAILURE
        }
    }
}

fn execute(arguments: &Arguments) -> Result<ExitCode, Failure> {
    if arguments.schema {
        let mut schema =
            serde_json::to_string_pretty(&schemars::schema_for!(configuration::Configuration))
                .map_err(|error| error.to_string())?;

        schema.push('\n');
        stdout(&schema)?;

        return Ok(ExitCode::SUCCESS);
    }

    let mut reporting = Reporting::new(arguments);

    if let Err(error) = format(arguments, &mut reporting) {
        reporting.abort(error);
    }

    reporting.finish()
}

fn format(arguments: &Arguments, reporting: &mut Reporting) -> Result<(), Failure> {
    let input = arguments.paths.iter().any(|path| path.as_os_str() == "-");

    if input && arguments.paths.len() != 1 {
        return Err("Stdin (-) must be the only input path".into());
    }

    if arguments.stdin_filepath.is_some() && !input {
        return Err("--stdin-filepath requires stdin (-) as the only input path".into());
    }

    let context = arguments
        .stdin_filepath
        .as_deref()
        .map(filepath)
        .transpose()?;

    let language = if input {
        Some(
            arguments
                .language
                .or_else(|| context.as_deref().and_then(Language::infer))
                .ok_or("Stdin requires --language (-l) or a recognizable --stdin-filepath (-f)")?,
        )
    } else {
        None
    };

    if let Some(language) = language {
        let directory = env::current_dir()?;

        let settings = Settings::load(
            arguments,
            context
                .as_deref()
                .and_then(Path::parent)
                .unwrap_or(&directory),
        )?;

        stdin(
            arguments,
            language,
            context.as_deref(),
            &settings.configuration,
            reporting,
        )
    } else {
        batch(arguments, reporting)
    }
}

fn filepath(path: &Path) -> Result<PathBuf, Failure> {
    let directory = env::current_dir()?;
    let path = directory.join(path);

    let parent = path
        .parent()
        .ok_or("Stdin filepath has no parent directory")?;

    let name = path
        .file_name()
        .ok_or("Stdin filepath requires a filename")?;

    let parent = fs::canonicalize(parent)
        .map_err(|error| Failure::from(error).context(&parent.display().to_string()))?;

    Ok(parent.join(name))
}

fn stdin(
    arguments: &Arguments,
    language: Language,
    context: Option<&Path>,
    configuration: &configuration::Configuration,
    reporting: &mut Reporting,
) -> Result<(), Failure> {
    let name = context.map_or_else(|| "stdin".into(), |path| path.display().to_string());

    reporting.progress(&format!("Checking 1/1  {name}"))?;

    let index = reporting.record(name.clone(), Status::Failed);

    let source = io::read_to_string(io::stdin().lock())
        .map_err(|error| Failure::from(error).context("stdin"))?;

    let formatted = language
        .format(&source, configuration)
        .map_err(|error| error.context(&name))?;

    let changed = source != formatted;

    reporting.files[index].status = if changed {
        Status::WouldChange
    } else {
        Status::Unchanged
    };

    if arguments.output.json {
        reporting.files[index].source = Some(formatted.clone());
    }

    if arguments.formatting.preview && changed {
        reporting.files[index].diff = Some(diff(&name, &source, &formatted));
    }

    let result = if arguments.output.json || arguments.formatting.check {
        Ok(())
    } else if arguments.formatting.preview {
        stdout(reporting.files[index].diff.as_deref().unwrap_or_default())
    } else {
        stdout(&formatted)
    };

    if let Err(error) = result {
        reporting.files[index].status = Status::Failed;

        return Err(error);
    }

    reporting.success(index)
}
