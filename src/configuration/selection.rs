use std::path::{Path, PathBuf};

use ignore::gitignore::{Gitignore, GitignoreBuilder};

use super::Configuration;
use crate::error::Failure;

pub(crate) struct Selection {
    root: PathBuf,
    include: Option<Gitignore>,
    exclude: Gitignore,
}

impl Selection {
    pub(crate) fn new(configuration: &Configuration, root: PathBuf) -> Result<Self, Failure> {
        fn compile(patterns: &[String], root: &Path) -> Result<Gitignore, Failure> {
            let mut builder = GitignoreBuilder::new(root);

            for pattern in patterns {
                builder
                    .add_line(None, pattern)
                    .map_err(|error| Failure::from(error.to_string()))?;
            }

            builder
                .build()
                .map_err(|error| Failure::from(error.to_string()))
        }

        let include = configuration
            .include
            .as_deref()
            .map(|patterns| compile(patterns, &root))
            .transpose()?;

        let exclude = compile(configuration.exclude.as_deref().unwrap_or_default(), &root)?;

        Ok(Self {
            root,
            include,
            exclude,
        })
    }

    pub(crate) fn excluded(&self, path: &Path, directory: bool) -> bool {
        path.strip_prefix(&self.root)
            .unwrap_or(path)
            .ancestors()
            .filter(|ancestor| !ancestor.as_os_str().is_empty())
            .enumerate()
            .any(|(depth, ancestor)| {
                self.exclude
                    .matched(ancestor, directory || depth != 0)
                    .is_ignore()
            })
    }

    pub(crate) fn includes(&self, path: &Path) -> bool {
        if self.excluded(path, false) {
            return false;
        }

        self.include.as_ref().is_none_or(|patterns| {
            let mut included = false;

            for (depth, ancestor) in path
                .strip_prefix(&self.root)
                .unwrap_or(path)
                .ancestors()
                .filter(|ancestor| !ancestor.as_os_str().is_empty())
                .enumerate()
            {
                let matched = patterns.matched(ancestor, depth != 0);

                if matched.is_whitelist() {
                    return false;
                }

                included |= matched.is_ignore();
            }

            included
        })
    }
}
