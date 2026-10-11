use std::path::{Path, PathBuf};

use globset::{Glob, GlobSet, GlobSetBuilder};

use super::Configuration;

pub(crate) struct Selection {
    root: PathBuf,
    include: Option<GlobSet>,
    exclude: GlobSet,
}

impl Selection {
    pub(crate) fn new(configuration: &Configuration, root: PathBuf) -> Result<Self, String> {
        fn compile(patterns: &[String]) -> Result<GlobSet, String> {
            let mut builder = GlobSetBuilder::new();

            for pattern in patterns {
                builder.add(Glob::new(pattern).map_err(|error| format!("{pattern}: {error}"))?);
            }

            builder.build().map_err(|error| error.to_string())
        }

        Ok(Self {
            root,
            include: configuration.include.as_deref().map(compile).transpose()?,
            exclude: compile(configuration.exclude.as_deref().unwrap_or_default())?,
        })
    }

    pub(crate) fn excluded(&self, path: &Path) -> bool {
        path.strip_prefix(&self.root)
            .unwrap_or(path)
            .ancestors()
            .any(|ancestor| self.exclude.is_match(ancestor))
    }

    pub(crate) fn excluded_directory(&self, path: &Path) -> bool {
        self.excluded(path)
            || self
                .exclude
                .is_match(path.strip_prefix(&self.root).unwrap_or(path).join(""))
    }

    pub(crate) fn includes(&self, path: &Path) -> bool {
        !self.excluded(path)
            && self.include.as_ref().is_none_or(|patterns| {
                patterns.is_match(path.strip_prefix(&self.root).unwrap_or(path))
            })
    }
}

#[cfg(test)]
mod tests {
    use super::{Configuration, Selection};

    #[test]
    fn filters_relative_paths_with_exclusion_precedence() {
        use std::path::PathBuf;
        let root = PathBuf::from("workspace");

        let configuration: Configuration = toml::from_str(
            "include = [\"src/**/*.{rs,luau}\"]\nexclude = [\"**/generated/**\", \"src/skip.rs\"]",
        )
        .unwrap();

        let selection = Selection::new(&configuration, root.clone()).unwrap();
        assert!(selection.includes(&root.join("src/main.rs")));
        assert!(selection.includes(&root.join("src/nested/main.luau")));
        assert!(!selection.includes(&root.join("src/main.lua")));
        assert!(!selection.includes(&root.join("other/main.rs")));
        assert!(!selection.includes(&root.join("src/skip.rs")));
        assert!(!selection.includes(&root.join("src/generated/main.rs")));
        assert!(selection.excluded_directory(&root.join("src/generated")));

        for (text, expected) in [
            ("", true),
            ("include = []", false),
            ("exclude = [\"src\"]", false),
        ] {
            let configuration = toml::from_str(text).unwrap();

            assert_eq!(
                Selection::new(&configuration, root.clone())
                    .unwrap()
                    .includes(&root.join("src/main.rs")),
                expected
            );
        }

        for text in ["include = [\"[\"]", "exclude = [\"[\"]"] {
            assert!(Selection::new(&toml::from_str(text).unwrap(), root.clone()).is_err());
        }
    }
}
