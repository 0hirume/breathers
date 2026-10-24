use std::{
    collections::BTreeMap,
    env, fs,
    path::{Path, PathBuf},
    sync::Arc,
};

use rayon::iter::{IntoParallelIterator as _, ParallelIterator as _};

use crate::{error::Failure, languages::Language};

use super::{
    arguments::Arguments,
    discovery::collect,
    preview::diff,
    reporting::{Reporting, Status, stdout},
    settings::Settings,
    writing,
};

pub(super) fn batch(arguments: &Arguments, reporting: &mut Reporting) -> Result<(), Failure> {
    let directory = env::current_dir()
        .and_then(fs::canonicalize)
        .map_err(Failure::from)?;

    reporting.progress("Scanning files...")?;

    let files = collect(arguments)?;

    if arguments.formatting.source && files.len() != 1 {
        return Err("--source requires exactly one selected file".into());
    }

    process(files, arguments, reporting, &directory)
}

fn process(
    files: BTreeMap<PathBuf, Arc<Settings>>,
    arguments: &Arguments,
    reporting: &mut Reporting,
    directory: &Path,
) -> Result<(), Failure> {
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
        .map(|(path, settings)| {
            let result = fs::read_to_string(&path)
                .map_err(Failure::from)
                .and_then(|source| {
                    let language = arguments
                        .language
                        .or_else(|| Language::infer(&path))
                        .ok_or("Unknown language; specify --language (-l)")?;

                    let output = language.format(&source, &settings.configuration);

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
        let (source, output) = result.map_err(|error| error.context(&name))?;

        let output = match output {
            Ok(output) => output,

            Err(error) => {
                reporting.error(&error.context(&name));

                continue;
            }
        };

        if source == output {
            reporting.files[index].status = Status::Unchanged;

            if arguments.formatting.source {
                changes.push((path, index, source, output));
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

            changes.push((path, index, source, output));
        }
    }

    let total = changes.len();

    for (position, (path, index, source, output)) in changes.into_iter().enumerate() {
        if writing {
            reporting.progress(&format!(
                "Writing {}/{total}  {}",
                position + 1,
                reporting.files[index].path,
            ))?;

            if let Err(error) = writing::replace(&path, &source, &output) {
                reporting.files[index].status = Status::Failed;

                return Err(Failure::from(error).context(&reporting.files[index].path));
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
