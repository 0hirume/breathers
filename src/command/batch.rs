use std::{
    collections::BTreeSet,
    env, fs,
    path::{Path, PathBuf},
};

use rayon::iter::{IntoParallelIterator as _, ParallelIterator as _};

use crate::{configuration, languages::Language};

use super::{
    arguments::Arguments,
    discovery::collect,
    preview::diff,
    reporting::{Reporting, Status, stdout},
};

pub(super) fn batch(
    arguments: &Arguments,
    configuration: &configuration::Configuration,
    selection: &configuration::Selection,
    reporting: &mut Reporting,
) -> Result<(), String> {
    let directory = env::current_dir()
        .and_then(fs::canonicalize)
        .map_err(|error| error.to_string())?;

    reporting.progress("Scanning files...")?;

    let mut files = BTreeSet::new();
    let mut directories = BTreeSet::new();

    for path in &arguments.paths {
        collect(path, &mut files, &mut directories, selection)?;
    }

    if arguments.formatting.source && files.len() != 1 {
        return Err("--source requires exactly one selected file".into());
    }

    process(files, arguments, configuration, reporting, &directory)
}

fn process(
    files: BTreeSet<PathBuf>,
    arguments: &Arguments,
    configuration: &configuration::Configuration,
    reporting: &mut Reporting,
    directory: &Path,
) -> Result<(), String> {
    let total = files.len();

    let writing = !arguments.formatting.check
        && !arguments.formatting.preview
        && !arguments.formatting.source;

    let mut changes = Vec::new();

    reporting.progress("Checking files...")?;

    let prepared: Vec<_> = files
        .into_iter()
        .collect::<Vec<_>>()
        .into_par_iter()
        .map(|path| {
            let result = fs::read_to_string(&path)
                .map_err(|error| error.to_string())
                .and_then(|source| {
                    let language = arguments
                        .language
                        .or_else(|| Language::infer(&path))
                        .ok_or("Unknown language; specify --language (-l)")?;

                    let output = language.format(&source, configuration, Some(&path));

                    Ok((source, output))
                });

            (path, result)
        })
        .collect();

    for (path, result) in prepared {
        let name = path
            .strip_prefix(directory)
            .unwrap_or(&path)
            .display()
            .to_string();

        reporting.progress(&format!(
            "Checking {}/{total}  {name}",
            reporting.files.len() + 1,
        ))?;

        let index = reporting.record(name.clone(), Status::Failed);
        let (source, output) = result.map_err(|error| format!("{name}\n  {error}"))?;

        let output = match output {
            Ok(output) => output,

            Err(error) => {
                reporting.error(format!(
                    "{name} (skipped)\n  {}",
                    error.replace('\n', "\n  "),
                ));

                continue;
            }
        };

        if source == output {
            reporting.files[index].status = Status::Unchanged;

            if arguments.formatting.source {
                changes.push((path, index, output));
            } else {
                reporting.success(index)?;
            }
        } else {
            reporting.files[index].status = if writing {
                Status::Unwritten
            } else {
                Status::WouldChange
            };

            if arguments.formatting.preview {
                reporting.files[index].diff = Some(diff(&name, &source, &output));
            }

            changes.push((path, index, output));
        }
    }

    let total = changes.len();

    for (position, (path, index, output)) in changes.into_iter().enumerate() {
        if writing {
            reporting.progress(&format!(
                "Writing {}/{total}  {}",
                position + 1,
                reporting.files[index].path,
            ))?;

            if let Err(error) = fs::write(&path, output) {
                reporting.files[index].status = Status::Failed;

                return Err(format!("{}\n  {error}", reporting.files[index].path));
            }

            reporting.files[index].status = Status::Formatted;
        } else {
            let result = if arguments.formatting.preview && !arguments.output.json {
                stdout(reporting.files[index].diff.as_deref().unwrap_or_default())
            } else if arguments.formatting.source {
                stdout(&output)
            } else {
                Ok(())
            };

            if let Err(error) = result {
                reporting.files[index].status = Status::Failed;

                return Err(error);
            }
        }

        reporting.success(index)?;
    }

    Ok(())
}
