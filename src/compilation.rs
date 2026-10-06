use std::{
    fs,
    path::{Path, PathBuf},
};

use regex::Regex;
use serde::Deserialize;

#[derive(Deserialize)]
#[serde(untagged)]
enum Strings {
    One(String),
    Many(Vec<String>),
}

impl Strings {
    fn values(&self) -> &[String] {
        match self {
            Self::One(value) => std::slice::from_ref(value),
            Self::Many(values) => values,
        }
    }

    fn matches(&self, path: &str) -> Result<bool, String> {
        let mut matched = false;

        for pattern in self.values() {
            matched |= Regex::new(&format!("^(?:{pattern})$"))
                .map_err(|error| error.to_string())?
                .is_match(path);
        }

        Ok(matched)
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "PascalCase", deny_unknown_fields)]
struct Conditions {
    path_match: Option<Strings>,
    path_exclude: Option<Strings>,
}

#[derive(Deserialize)]
#[serde(rename_all = "PascalCase", deny_unknown_fields)]
struct Flags {
    add: Option<Strings>,
}

#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
struct Fragment {
    #[serde(rename = "If")]
    conditions: Option<Conditions>,

    compile_flags: Option<Flags>,
}

fn extend(source: &str, relative: &str, arguments: &mut Vec<String>) -> Result<(), String> {
    let fragments: Vec<Option<Fragment>> =
        serde_saphyr::from_multiple(source).map_err(|error| error.to_string())?;

    for fragment in fragments.into_iter().flatten() {
        if let Some(conditions) = fragment.conditions {
            if let Some(patterns) = conditions.path_match
                && !patterns.matches(relative)?
            {
                continue;
            }

            if let Some(patterns) = conditions.path_exclude
                && patterns.matches(relative)?
            {
                continue;
            }
        }

        if let Some(flags) = fragment.compile_flags
            && let Some(add) = flags.add
        {
            arguments.extend_from_slice(add.values());
        }
    }

    Ok(())
}

fn database(path: &Path) -> Result<Option<(PathBuf, Vec<String>)>, String> {
    let source = fs::canonicalize(path).unwrap_or_else(|_| path.to_owned());

    for directory in path.ancestors().skip(1) {
        let configuration = directory.join("compile_commands.json");

        if !configuration
            .try_exists()
            .map_err(|error| format!("{}: {error}", configuration.display()))?
        {
            continue;
        }

        let contents = fs::read(&configuration)
            .map_err(|error| format!("{}: {error}", configuration.display()))?;

        let entries: Vec<serde_json::Value> = serde_json::from_slice(&contents)
            .map_err(|error| format!("{}: {error}", configuration.display()))?;

        let database = clang::CompilationDatabase::from_directory(directory).map_err(|()| {
            format!(
                "{}: could not load compilation database",
                configuration.display()
            )
        })?;

        if database.get_all_compile_commands().get_commands().len() != entries.len() {
            return Err(format!(
                "{}: invalid compilation commands",
                configuration.display()
            ));
        }

        let Ok(commands) = database.get_compile_commands(path) else {
            return Ok(None);
        };

        let commands = commands.get_commands();

        let Some(command) = commands.first() else {
            return Ok(None);
        };

        let directory = directory.join(command.get_directory());
        let mut arguments = command.get_arguments().into_iter();

        let compiler = arguments
            .next()
            .ok_or("Compilation command has no compiler")?;

        let compiler = Path::new(&compiler)
            .file_stem()
            .and_then(|name| name.to_str());

        let mut flags = Vec::new();

        if matches!(compiler, Some("cl" | "clang-cl")) {
            flags.push("--driver-mode=cl".to_owned());
        }

        for argument in arguments {
            let candidate = directory.join(&argument);

            if argument != "--" && fs::canonicalize(&candidate).unwrap_or(candidate) != source {
                flags.push(argument);
            }
        }

        return Ok(Some((directory, flags)));
    }

    Ok(None)
}

pub fn arguments(path: &Path) -> Result<Vec<String>, String> {
    let directory = path.parent().ok_or("Source path has no parent directory")?;

    let (working_directory, mut arguments) =
        database(path)?.unwrap_or_else(|| (directory.to_owned(), Vec::new()));

    let prefix = if arguments
        .iter()
        .any(|argument| argument == "--driver-mode=cl")
    {
        "/clang:"
    } else {
        ""
    };

    arguments.push(format!(
        "{prefix}-working-directory={}",
        working_directory.display()
    ));

    let ancestors: Vec<_> = directory.ancestors().collect();

    for directory in ancestors.into_iter().rev() {
        let configuration = directory.join(".clangd");

        let source = match fs::read_to_string(&configuration) {
            Ok(source) => source,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => continue,
            Err(error) => return Err(format!("{}: {error}", configuration.display())),
        };

        let relative = path
            .strip_prefix(directory)
            .map_err(|error| error.to_string())?
            .to_string_lossy()
            .replace('\\', "/");

        extend(&source, &relative, &mut arguments)
            .map_err(|error| format!("{}: {error}", configuration.display()))?;
    }

    Ok(arguments)
}

#[cfg(test)]
mod tests {
    #[test]
    fn reads_conditional_yaml_documents() {
        let source = "If:\n  PathMatch: ['bridge/.*', 'other/.*']\n  PathExclude: 'bridge/generated/.*'\nCompileFlags:\n  Add:\n    - -std=c++17\n    - '-I../headers with spaces'\n---\nCompileFlags:\n  Add: -DSECOND\n";
        let mut arguments = Vec::new();
        super::extend(source, "bridge/source.cpp", &mut arguments).unwrap();

        assert_eq!(
            arguments,
            ["-std=c++17", "-I../headers with spaces", "-DSECOND"]
        );

        arguments.clear();
        super::extend(source, "bridge/generated/source.cpp", &mut arguments).unwrap();
        assert_eq!(arguments, ["-DSECOND"]);
        assert!(super::extend("CompileFlags: [broken", "source.cpp", &mut arguments).is_err());

        assert!(
            super::extend(
                "CompileFlags:\n  Remove: -I\n",
                "source.cpp",
                &mut arguments
            )
            .is_err()
        );
    }
}
