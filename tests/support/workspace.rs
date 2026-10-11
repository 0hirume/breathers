use std::{
    env::temp_dir,
    fs,
    io::{ErrorKind, Write as _},
    path::{Path, PathBuf},
    process::{Command, Output, Stdio, id},
    sync::atomic::{AtomicU64, Ordering},
    thread::panicking,
    time::{SystemTime, UNIX_EPOCH},
};

static SEQUENCE: AtomicU64 = AtomicU64::new(0);

pub(crate) struct Workspace {
    root: PathBuf,
}

impl Workspace {
    pub(crate) fn new() -> Self {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();

        let workspace = loop {
            let sequence = SEQUENCE.fetch_add(1, Ordering::Relaxed);
            let root = temp_dir().join(format!("breathers-{}-{timestamp}-{sequence}", id()));

            match fs::create_dir(&root) {
                Ok(()) => break Self { root },
                Err(error) if error.kind() == ErrorKind::AlreadyExists => {}
                Err(error) => panic!("Cannot create {}: {error}", root.display()),
            }
        };

        fs::create_dir(workspace.root.join(".application")).unwrap();
        fs::create_dir(workspace.root.join(".configuration")).unwrap();
        fs::create_dir(workspace.root.join(".home")).unwrap();
        fs::write(workspace.root.join("breathers.toml"), "").unwrap();

        workspace
    }

    pub(crate) fn path(&self) -> &Path {
        &self.root
    }

    pub(crate) fn command(&self) -> Command {
        let mut command = Command::new(env!("CARGO_BIN_EXE_breathers"));

        command
            .current_dir(&self.root)
            .env("APPDATA", self.root.join(".application"))
            .env("XDG_CONFIG_HOME", self.root.join(".configuration"))
            .env("HOME", self.root.join(".home"));

        command
    }

    pub(crate) fn input(&self, arguments: &[&str], source: &[u8]) -> Output {
        let mut child = self
            .command()
            .args(arguments)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();

        child.stdin.take().unwrap().write_all(source).unwrap();

        child.wait_with_output().unwrap()
    }
}

impl Drop for Workspace {
    fn drop(&mut self) {
        if let Err(error) = fs::remove_dir_all(&self.root) {
            if panicking() {
                eprintln!(
                    "Cannot remove {} during unwinding: {error}",
                    self.root.display()
                );
            } else {
                panic!("Cannot remove {}: {error}", self.root.display());
            }
        }
    }
}
