use std::{
    collections::BTreeSet,
    env, fs,
    path::{Path, PathBuf},
    process::ExitCode,
    time::{Duration, Instant},
};

use console::style;
use indicatif::{ProgressBar, ProgressStyle};

use crate::{configuration, languages::Language};

use super::{arguments::Arguments, discovery::collect};

#[derive(Default)]
struct Summary {
    processed: usize,
    changed: usize,
    unchanged: usize,
    failed: usize,
}

pub(super) fn batch(
    arguments: Arguments,
    configuration: &configuration::Configuration,
    selection: &configuration::Selection,
) -> Result<ExitCode, String> {
    let started = Instant::now();

    let directory = env::current_dir()
        .and_then(fs::canonicalize)
        .map_err(|error| error.to_string())?;

    let indicator = ProgressBar::new_spinner().with_style(
        ProgressStyle::with_template("{spinner:.cyan} {msg}").map_err(|error| error.to_string())?,
    );

    progress(&indicator, "Scanning files...".into());

    if !indicator.is_hidden() {
        indicator.enable_steady_tick(Duration::from_millis(120));
    }

    let mut summary = Summary::default();

    let result = (|| {
        let mut files = BTreeSet::new();
        let mut directories = BTreeSet::new();

        for path in arguments.paths {
            collect(&path, &mut files, &mut directories, selection)?;
        }

        process(
            files,
            arguments.language,
            configuration,
            &indicator,
            &mut summary,
            &directory,
        )
    })();

    indicator.finish_and_clear();

    if let Err(error) = &result {
        report(error);
        eprintln!("{}", style("Aborted").for_stderr().red());
    }

    let failures = style(summary.failed).for_stderr();

    let failures = if summary.failed == 0 {
        failures.dim()
    } else {
        failures.red()
    };

    eprintln!(
        "{} processed · {} changed · {} unchanged · {} failed · {:?} elapsed",
        summary.processed,
        style(summary.changed).for_stderr().green(),
        style(summary.unchanged).for_stderr().dim(),
        failures,
        style(started.elapsed()).for_stderr().cyan(),
    );

    let pending = summary.processed - summary.changed - summary.unchanged - summary.failed;

    if pending > 0 {
        eprintln!("{pending} prepared changes not written");
    }

    Ok(if result.is_ok() && summary.failed == 0 {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    })
}

pub(crate) fn report(error: &str) {
    eprintln!("{} {error}", style("error:").for_stderr().red().bold());
}

fn progress(progress: &ProgressBar, message: String) {
    if progress.is_hidden() {
        eprintln!("{message}");
    }

    progress.set_message(message);
}

fn process(
    files: BTreeSet<PathBuf>,
    language: Option<Language>,
    configuration: &configuration::Configuration,
    indicator: &ProgressBar,
    summary: &mut Summary,
    directory: &Path,
) -> Result<(), String> {
    let total = files.len();
    let mut changes = Vec::new();

    for path in files {
        let name = path
            .strip_prefix(directory)
            .unwrap_or(&path)
            .display()
            .to_string();

        summary.processed += 1;

        progress(
            indicator,
            format!("Checking {}/{total}  {name}", summary.processed),
        );

        let source = fs::read_to_string(&path).map_err(|error| {
            summary.failed += 1;

            format!("{name}\n  {error}")
        })?;

        let language = language.or_else(|| Language::infer(&path)).ok_or_else(|| {
            summary.failed += 1;

            format!("{name}\n  Unknown language; specify --language (-l)")
        })?;

        let output = match language.format(&source, configuration, Some(&path)) {
            Ok(output) => output,

            Err(error) => {
                summary.failed += 1;

                indicator.suspend(|| {
                    report(&format!(
                        "{name} (skipped)\n  {}",
                        error.replace('\n', "\n  ")
                    ));
                });

                continue;
            }
        };

        if source == output {
            summary.unchanged += 1;
            indicator.suspend(|| eprintln!("{}  {name}", style("Unchanged").for_stderr().dim()));
        } else {
            changes.push((path, name, output));
        }
    }

    let total = changes.len();

    for (path, name, output) in changes {
        progress(
            indicator,
            format!("Writing {}/{total}  {name}", summary.changed + 1),
        );

        fs::write(&path, output).map_err(|error| {
            summary.failed += 1;

            format!("{name}\n  {error}")
        })?;

        summary.changed += 1;
        indicator.suspend(|| eprintln!("{}  {name}", style("Formatted").for_stderr().green()));
    }

    Ok(())
}
