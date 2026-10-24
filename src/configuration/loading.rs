use std::{
    env, fs,
    io::ErrorKind,
    path::{Path, PathBuf},
};

use miette::LabeledSpan;
use toml::de::Error;

use super::Configuration;
use crate::error::Failure;

pub(crate) fn load_in(
    directory: &Path,
    path: Option<&Path>,
) -> Result<(Configuration, PathBuf), Failure> {
    let configuration_directory = if cfg!(windows) {
        env::var_os("APPDATA")
            .filter(|directory| !directory.is_empty())
            .map(PathBuf::from)
            .or_else(dirs::config_dir)
    } else {
        dirs::config_dir()
    };

    let global = configuration_directory.map(|directory| directory.join("breathers/config.toml"));

    load_from(directory, global.as_deref(), path)
}

fn load_from(
    directory: &Path,
    global: Option<&Path>,
    explicit: Option<&Path>,
) -> Result<(Configuration, PathBuf), Failure> {
    let mut configuration = global.map(read).transpose()?.flatten().unwrap_or_default();

    let mut root = env::current_dir()?;

    if let Some(path) = explicit {
        let path = root.join(path);

        let project =
            read(&path)?.ok_or_else(|| format!("{}: configuration not found", path.display()))?;

        configuration.overlay(project);

        root = path
            .parent()
            .ok_or("Configuration has no parent directory")?
            .to_path_buf();
    } else {
        for ancestor in directory.ancestors() {
            if let Some(project) = read(&ancestor.join("breathers.toml"))? {
                configuration.overlay(project);
                root = ancestor.to_path_buf();
                break;
            }
        }
    }

    let root = fs::canonicalize(&root)
        .map_err(|error| Failure::from(error).context(&root.display().to_string()))?;

    Ok((configuration, root))
}

fn read(path: &Path) -> Result<Option<Configuration>, Failure> {
    match fs::read_to_string(path) {
        Ok(source) => toml::from_str(&source).map(Some).map_err(|error: Error| {
            let span = error.span().unwrap_or(0..0);

            Failure::syntax(
                &source,
                vec![LabeledSpan::new_with_span(
                    Some(error.message().to_owned()),
                    span,
                )],
            )
            .context(&path.display().to_string())
        }),

        Err(error) if error.kind() == ErrorKind::NotFound => Ok(None),
        Err(error) => Err(Failure::from(error).context(&path.display().to_string())),
    }
}
