use std::{
    env, fs,
    io::ErrorKind,
    path::{Path, PathBuf},
};

use super::Configuration;

pub(crate) fn load(path: Option<&Path>) -> Result<(Configuration, PathBuf), String> {
    let directory = env::current_dir().map_err(|error| error.to_string())?;

    load_in(&directory, path)
}

pub(crate) fn load_in(
    directory: &Path,
    path: Option<&Path>,
) -> Result<(Configuration, PathBuf), String> {
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
) -> Result<(Configuration, PathBuf), String> {
    let mut configuration = global.map(read).transpose()?.flatten().unwrap_or_default();

    let mut root = directory.to_path_buf();

    if let Some(path) = explicit {
        let path = directory.join(path);

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

    let root = fs::canonicalize(&root).map_err(|error| format!("{}: {error}", root.display()))?;

    Ok((configuration, root))
}

fn read(path: &Path) -> Result<Option<Configuration>, String> {
    match fs::read_to_string(path) {
        Ok(source) => toml::from_str(&source)
            .map(Some)
            .map_err(|error| format!("{}: {error}", path.display())),

        Err(error) if error.kind() == ErrorKind::NotFound => Ok(None),
        Err(error) => Err(format!("{}: {error}", path.display())),
    }
}

#[cfg(test)]
mod tests {
    use std::{
        env, fs, process,
        time::{SystemTime, UNIX_EPOCH},
    };

    use super::load_from;
    use crate::configuration::Rule;

    #[test]
    fn layers_global_and_nearest_project_settings() {
        let root = env::temp_dir()
            .join("breathers")
            .join(process::id().to_string())
            .join(
                SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_nanos()
                    .to_string(),
            );

        let project = root.join("project");
        let nested = project.join("nested");
        fs::create_dir_all(&nested).unwrap();
        let global = root.join("global.toml");

        fs::write(
            &global,
            r#"include = ["**/*.rs"]
exclude = ["vendor/**"]
[rust]
returns = false
tail_expressions = true
related = true
[python]
blocks = false
"#,
        )
        .unwrap();

        fs::write(
            project.join("breathers.toml"),
            r"[rust]
returns = true
related = false
[python]
conditionals = true
",
        )
        .unwrap();

        let (configuration, selected) = load_from(&nested, Some(&global), None).unwrap();

        assert_eq!(selected, fs::canonicalize(&project).unwrap());
        assert_eq!(configuration.include.unwrap(), ["**/*.rs"]);
        assert_eq!(configuration.exclude.unwrap(), ["vendor/**"]);
        assert!(configuration.rust.enabled(Rule::ReturnStatements));
        assert!(configuration.rust.enabled(Rule::TailExpressions));
        assert!(!configuration.rust.enabled(Rule::Related));
        assert!(!configuration.python.enabled(Rule::Functions));
        assert!(configuration.python.enabled(Rule::Conditionals));

        fs::write(
            nested.join("breathers.toml"),
            r"include = []
exclude = []
[rust]
fields = false
",
        )
        .unwrap();

        let (configuration, selected) = load_from(&nested, Some(&global), None).unwrap();

        assert_eq!(selected, fs::canonicalize(&nested).unwrap());
        assert_eq!(configuration.include.unwrap(), Vec::<String>::new());
        assert_eq!(configuration.exclude.unwrap(), Vec::<String>::new());
        assert!(!configuration.rust.enabled(Rule::ReturnStatements));
        assert!(configuration.rust.enabled(Rule::Related));
        let explicit = root.join("selected.toml");
        fs::write(&explicit, "[rust]\nreturns = true\n").unwrap();

        let (configuration, selected) = load_from(&nested, Some(&global), Some(&explicit)).unwrap();

        assert_eq!(selected, fs::canonicalize(&root).unwrap());
        assert!(configuration.rust.enabled(Rule::ReturnStatements));
        assert!(configuration.rust.enabled(Rule::StructFields));
        assert!(!configuration.python.enabled(Rule::Functions));

        assert!(load_from(&nested, Some(&global), Some(&root.join("missing.toml"))).is_err());

        fs::write(&global, "[rust]\nreturns = 1\n").unwrap();
        assert!(load_from(&nested, Some(&global), Some(&explicit)).is_err());
        fs::remove_file(&global).unwrap();

        assert!(
            load_from(&nested, Some(&global), Some(&explicit))
                .unwrap()
                .0
                .python
                .enabled(Rule::Functions)
        );

        fs::remove_dir_all(root).unwrap();
    }
}
