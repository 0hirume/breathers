use std::{
    collections::BTreeSet,
    fs,
    path::{Path, PathBuf},
};

use crate::{configuration::Selection, languages::Language};

pub(super) fn collect(
    path: &Path,
    files: &mut BTreeSet<PathBuf>,
    directories: &mut BTreeSet<PathBuf>,
    selection: &Selection,
) -> Result<(), String> {
    let canonical =
        fs::canonicalize(path).map_err(|error| format!("{}: {error}", path.display()))?;

    let metadata =
        fs::metadata(&canonical).map_err(|error| format!("{}: {error}", path.display()))?;

    if metadata.is_dir() {
        if selection.excluded_directory(&canonical) || !directories.insert(canonical.clone()) {
            return Ok(());
        }

        for entry in
            fs::read_dir(&canonical).map_err(|error| format!("{}: {error}", path.display()))?
        {
            let entry = entry.map_err(|error| format!("{}: {error}", path.display()))?;

            let kind = entry
                .file_type()
                .map_err(|error| format!("{}: {error}", entry.path().display()))?;

            if kind.is_symlink() || (kind.is_dir() && entry.file_name() == ".git") {
                continue;
            }

            if kind.is_dir() || (kind.is_file() && Language::infer(&entry.path()).is_some()) {
                collect(&entry.path(), files, directories, selection)?;
            }
        }
    } else if metadata.is_file() {
        if selection.includes(&canonical) {
            files.insert(canonical);
        }
    } else {
        return Err(format!("{}: expected a file or directory", path.display()));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use std::{
        collections::BTreeSet,
        env, fs, process,
        time::{SystemTime, UNIX_EPOCH},
    };

    use crate::configuration::{Configuration, Selection};

    use super::collect;

    #[test]
    fn discovers_source_files_and_deduplicates_paths() {
        let root = env::temp_dir().join(format!(
            "breathers-{}-{}",
            process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));

        fs::create_dir(&root).unwrap();

        for directory in ["src", "src/nested", "target", ".git"] {
            fs::create_dir(root.join(directory)).unwrap();
        }

        for file in [
            "root.rs",
            "src/main.rs",
            "src/nested/module.rs",
            "src/main.c",
            "src/header.h",
            "src/main.cpp",
            "src/header.hpp",
            "src/notes.txt",
            "target/generated.rs",
            ".git/ignored.rs",
        ] {
            fs::write(root.join(file), "fn example() {}\n").unwrap();
        }

        let expected: BTreeSet<_> = [
            "root.rs",
            "src/main.rs",
            "src/nested/module.rs",
            "src/main.c",
            "src/header.h",
            "src/main.cpp",
            "src/header.hpp",
            "target/generated.rs",
        ]
        .map(|path| fs::canonicalize(root.join(path)).unwrap())
        .into_iter()
        .collect();

        let mut files = BTreeSet::new();
        let mut directories = BTreeSet::new();

        let selection = Selection::new(&Configuration::default(), root.clone()).unwrap();

        collect(&root, &mut files, &mut directories, &selection).unwrap();
        collect(&root.join("src"), &mut files, &mut directories, &selection).unwrap();

        collect(
            &root.join("src/main.rs"),
            &mut files,
            &mut directories,
            &selection,
        )
        .unwrap();

        assert_eq!(files, expected);
        files.clear();
        directories.clear();
        collect(&root.join("src"), &mut files, &mut directories, &selection).unwrap();
        assert_eq!(files.len(), 6);
        files.clear();

        collect(
            &root.join("src/main.rs"),
            &mut files,
            &mut directories,
            &selection,
        )
        .unwrap();

        assert_eq!(
            files,
            BTreeSet::from([fs::canonicalize(root.join("src/main.rs")).unwrap()])
        );

        assert!(
            collect(
                &root.join("missing"),
                &mut files,
                &mut directories,
                &selection
            )
            .is_err()
        );

        fs::remove_dir_all(root).unwrap();
    }
}
