use std::{fs, path::Path};

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

pub fn arguments(path: &Path) -> Result<Vec<String>, String> {
    let directory = path.parent().ok_or("Source path has no parent directory")?;
    let mut arguments = Vec::new();
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
