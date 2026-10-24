use std::{
    collections::{BTreeMap, BTreeSet, btree_map::Entry},
    fs,
    path::{Path, PathBuf},
    sync::Arc,
};

use ignore::{IncrementalIgnore, WalkBuilder};

use crate::{error::Failure, languages::Language};

use super::{arguments::Arguments, settings::Settings};

struct Discovery<'arguments> {
    arguments: &'arguments Arguments,
    files: BTreeMap<PathBuf, Arc<Settings>>,
    directories: BTreeSet<PathBuf>,
    settings: BTreeMap<PathBuf, Arc<Settings>>,
}

impl Discovery<'_> {
    fn collect(&mut self, path: &Path, ignores: &mut IncrementalIgnore) -> Result<(), Failure> {
        let metadata = fs::metadata(path)
            .map_err(|error| Failure::from(error).context(&path.display().to_string()))?;

        if !metadata.is_dir() && !metadata.is_file() {
            return Err(format!("{}: expected a file or directory", path.display()).into());
        }

        if metadata.is_dir()
            && path.file_name().is_some_and(|name| {
                matches!(
                    name.to_str(),
                    Some(".git" | "target" | ".venv" | "node_modules")
                )
            })
        {
            return Ok(());
        }

        let relative = path
            .strip_prefix(ignores.root())
            .map_err(|error| Failure::from(error.to_string()))?;

        let (matched, error) = ignores.matched_with_errors(relative, metadata.is_dir());

        if let Some(error) = error {
            return Err(Failure::from(error.to_string()).context(&path.display().to_string()));
        }

        if matched.is_ignore() {
            return Ok(());
        }

        let directory = if metadata.is_dir() {
            path
        } else {
            path.parent().ok_or("Source file has no parent directory")?
        };

        let settings = self.settings(directory)?;

        if metadata.is_dir() {
            if settings.selection.excluded(path, true)
                || !self.directories.insert(path.to_path_buf())
            {
                return Ok(());
            }

            for entry in fs::read_dir(path)
                .map_err(|error| Failure::from(error).context(&path.display().to_string()))?
            {
                let entry = entry
                    .map_err(|error| Failure::from(error).context(&path.display().to_string()))?;

                let kind = entry.file_type().map_err(|error| {
                    Failure::from(error).context(&entry.path().display().to_string())
                })?;

                if kind.is_symlink() {
                    continue;
                }

                if (kind.is_dir() && !settings.selection.excluded(&entry.path(), true))
                    || (kind.is_file() && Language::infer(&entry.path()).is_some())
                {
                    self.collect(&entry.path(), ignores)?;
                }
            }
        } else if settings.selection.includes(path) {
            self.files.insert(path.to_path_buf(), settings);
        }

        Ok(())
    }

    fn settings(&mut self, directory: &Path) -> Result<Arc<Settings>, Failure> {
        Ok(match self.settings.entry(directory.to_path_buf()) {
            Entry::Occupied(entry) => Arc::clone(entry.get()),

            Entry::Vacant(entry) => {
                Arc::clone(entry.insert(Arc::new(Settings::load(self.arguments, directory)?)))
            }
        })
    }
}

pub(super) fn collect(arguments: &Arguments) -> Result<BTreeMap<PathBuf, Arc<Settings>>, Failure> {
    let mut discovery = Discovery {
        arguments,
        files: BTreeMap::new(),
        directories: BTreeSet::new(),
        settings: BTreeMap::new(),
    };

    for path in &arguments.paths {
        let canonical = fs::canonicalize(path)
            .map_err(|error| Failure::from(error).context(&path.display().to_string()))?;

        let directory = if canonical.is_dir() {
            canonical.as_path()
        } else {
            canonical
                .parent()
                .ok_or("Source file has no parent directory")?
        };

        discovery.settings(directory)?;

        let root = if canonical.is_dir() {
            canonical.parent().unwrap_or(&canonical)
        } else {
            &canonical
        };

        let mut ignores = WalkBuilder::new(root)
            .hidden(false)
            .ignore(false)
            .git_global(false)
            .git_exclude(false)
            .require_git(false)
            .build_matchers()
            .pop()
            .ok_or("Missing Git ignore matcher")?;

        discovery.collect(&canonical, &mut ignores)?;
    }

    Ok(discovery.files)
}
