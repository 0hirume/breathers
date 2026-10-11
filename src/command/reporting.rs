use std::{
    env,
    io::{self, IsTerminal as _, Write as _},
    process::ExitCode,
    time::{Duration, Instant},
};

use console::style;
use indicatif::ProgressBar;
use serde::Serialize;

use super::arguments::{Arguments, Color};

#[derive(Copy, Clone, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub(super) enum Status {
    Formatted,
    WouldChange,
    Unchanged,
    Failed,
    Unwritten,
}

#[derive(Serialize)]
pub(super) struct Record {
    pub(super) path: String,
    pub(super) status: Status,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub(super) source: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub(super) diff: Option<String>,
}

#[derive(Default, Serialize)]
struct Summary {
    processed: usize,
    changed: usize,
    unchanged: usize,
    failed: usize,
    would_change: usize,
    unwritten: usize,
}

#[derive(Copy, Clone, Eq, PartialEq)]
enum Verbosity {
    Normal,
    Quiet,
    Verbose,
}

pub(super) struct Reporting {
    pub(super) files: Vec<Record>,
    mode: &'static str,
    errors: Vec<String>,
    json: bool,
    verbosity: Verbosity,
    indicator: ProgressBar,
    colors: bool,
    aborted: bool,
    started: Instant,
}

impl Reporting {
    pub(super) fn new(arguments: &Arguments) -> Self {
        let input = arguments.paths.iter().any(|path| path.as_os_str() == "-");

        let mode = if arguments.formatting.check {
            "check"
        } else if arguments.formatting.preview {
            "preview"
        } else if arguments.formatting.source || input {
            "source"
        } else {
            "write"
        };

        let verbosity = if arguments.output.quiet {
            Verbosity::Quiet
        } else if arguments.output.verbose {
            Verbosity::Verbose
        } else {
            Verbosity::Normal
        };

        let indicator = if verbosity == Verbosity::Verbose
            && !arguments.output.json
            && io::stderr().is_terminal()
        {
            ProgressBar::new_spinner()
        } else {
            ProgressBar::hidden()
        };

        if !indicator.is_hidden() {
            indicator.enable_steady_tick(Duration::from_millis(120));
        }

        Self {
            files: Vec::new(),
            mode,
            errors: Vec::new(),
            json: arguments.output.json,
            verbosity,
            indicator,
            colors: colors(arguments.output.color),
            aborted: false,
            started: Instant::now(),
        }
    }

    pub(super) fn record(&mut self, path: String, status: Status) -> usize {
        let index = self.files.len();

        self.files.push(Record {
            path,
            status,
            source: None,
            diff: None,
        });

        index
    }

    pub(super) fn progress(&self, message: &str) -> Result<(), String> {
        if self.verbosity == Verbosity::Verbose && !self.json {
            if self.indicator.is_hidden() {
                stderr(message)?;
            } else {
                self.indicator.set_message(message.to_owned());
            }
        }

        Ok(())
    }

    pub(super) fn success(&self, index: usize) -> Result<(), String> {
        let record = &self.files[index];

        if self.verbosity == Verbosity::Quiet
            || self.json
            || (self.mode == "source" && self.verbosity != Verbosity::Verbose)
        {
            return Ok(());
        }

        let label = match record.status {
            Status::Formatted => style("Formatted").green(),
            Status::WouldChange => style("Would change").yellow(),
            Status::Unchanged if self.verbosity == Verbosity::Verbose => style("Unchanged").dim(),
            Status::Unchanged | Status::Failed | Status::Unwritten => return Ok(()),
        };

        self.indicator.suspend(|| {
            stderr(&format!(
                "{}  {}",
                label.for_stderr().force_styling(self.colors),
                record.path,
            ))
        })
    }

    pub(super) fn error(&mut self, error: String) {
        if let Err(output) = self.indicator.suspend(|| report(&error, self.colors)) {
            self.errors.push(output);
        }

        self.errors.push(error);
    }

    pub(super) fn abort(&mut self, error: String) {
        self.aborted = true;

        let unwritten = self
            .files
            .iter()
            .filter(|file| file.status == Status::Unwritten)
            .count();

        let error = if unwritten > 0 {
            format!("{error}\n  {unwritten} prepared changes not written")
        } else {
            error
        };

        self.error(error);
    }

    pub(super) fn finish(self) -> Result<ExitCode, String> {
        self.indicator.finish_and_clear();

        let mut summary = Summary::default();

        for file in &self.files {
            summary.processed += 1;

            match file.status {
                Status::Formatted => summary.changed += 1,
                Status::WouldChange => summary.would_change += 1,
                Status::Unchanged => summary.unchanged += 1,
                Status::Failed => summary.failed += 1,
                Status::Unwritten => summary.unwritten += 1,
            }
        }

        if self.json {
            let files: Vec<_> = self
                .files
                .iter()
                .filter(|file| {
                    self.verbosity == Verbosity::Verbose
                        || file.status != Status::Unchanged
                        || file.source.is_some()
                })
                .collect();

            let document = serde_json::json!({
                "mode": self.mode,
                "files": files,
                "summary": summary,
                "errors": self.errors,
            });

            let mut output = serde_json::to_string(&document).map_err(|error| error.to_string())?;
            output.push('\n');
            stdout(&output)?;
        } else if self.verbosity != Verbosity::Quiet
            && (self.mode != "source" || self.verbosity == Verbosity::Verbose)
        {
            let changed = if self.mode == "write" {
                format!("{} changed", summary.changed)
            } else {
                format!("{} would change", summary.would_change)
            };

            let mut message = format!(
                "{} processed · {changed} · {} unchanged · {} failed · {:?} elapsed",
                summary.processed,
                summary.unchanged,
                summary.failed,
                self.started.elapsed(),
            );

            if self.aborted {
                message.push_str(" · Aborted");
            }

            stderr(&message)?;
        }

        Ok(
            if self.errors.is_empty()
                && summary.failed == 0
                && !(self.mode == "check" && summary.would_change > 0)
            {
                ExitCode::SUCCESS
            } else {
                ExitCode::FAILURE
            },
        )
    }
}

pub(super) fn colors(color: Color) -> bool {
    match color {
        Color::Auto => io::stderr().is_terminal() && env::var_os("NO_COLOR").is_none(),
        Color::Always => true,
        Color::Never => false,
    }
}

pub(super) fn report(error: &str, colors: bool) -> Result<(), String> {
    stderr(&format!(
        "{} {error}",
        style("error:")
            .for_stderr()
            .red()
            .bold()
            .force_styling(colors),
    ))
}

pub(super) fn stdout(source: &str) -> Result<(), String> {
    let mut output = io::stdout().lock();

    output
        .write_all(source.as_bytes())
        .and_then(|()| output.flush())
        .map_err(|error| format!("stdout: {error}"))
}

fn stderr(message: &str) -> Result<(), String> {
    let mut output = io::stderr().lock();

    writeln!(output, "{message}")
        .and_then(|()| output.flush())
        .map_err(|error| format!("stderr: {error}"))
}
