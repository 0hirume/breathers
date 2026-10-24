use std::{
    fs::{self, File, OpenOptions, Permissions},
    io::{self, Read as _, Write as _},
    path::Path,
};

use same_file::Handle;
use tempfile::NamedTempFile;

pub(super) fn replace(path: &Path, original: &str, proposed: &str) -> io::Result<()> {
    let (mut file, permissions) = locked(path)?;

    let parent = path
        .parent()
        .ok_or_else(|| io::Error::other("Write destination has no parent directory"))?;

    let mut staged = NamedTempFile::new_in(parent)?;
    staged.write_all(proposed.as_bytes())?;
    staged.as_file().set_permissions(permissions.clone())?;
    staged.as_file().sync_all()?;
    validate(&mut file, path, original, &permissions)?;
    fs::rename(staged.path(), path)?;
    staged.disable_cleanup(true);

    Ok(())
}

fn locked(path: &Path) -> io::Result<(File, Permissions)> {
    let metadata = fs::symlink_metadata(path)?;

    if !metadata.is_file() || metadata.permissions().readonly() {
        return Err(io::Error::other(
            "Cannot replace a nonregular or read-only file",
        ));
    }

    let file = OpenOptions::new().read(true).write(true).open(path)?;
    file.try_lock().map_err(io::Error::other)?;

    if !file.metadata()?.is_file() {
        return Err(io::Error::other("Cannot replace a nonregular file"));
    }

    Ok((file, metadata.permissions()))
}

fn validate(
    file: &mut File,
    path: &Path,
    original: &str,
    permissions: &Permissions,
) -> io::Result<()> {
    let mut current = Vec::new();

    (&mut *file)
        .take(
            u64::try_from(original.len())
                .map_err(io::Error::other)?
                .saturating_add(1),
        )
        .read_to_end(&mut current)?;

    if current != original.as_bytes() {
        return Err(io::Error::other("File contents changed before replacement"));
    }

    let metadata = fs::symlink_metadata(path)?;

    if !metadata.is_file() || metadata.permissions() != *permissions {
        return Err(io::Error::other(
            "Write destination changed before replacement",
        ));
    }

    if Handle::from_file(file.try_clone()?)? != Handle::from_path(path)? {
        return Err(io::Error::other(
            "Write destination changed before replacement",
        ));
    }

    Ok(())
}
