use std::{
    fs,
    io::{self, Write as _},
    panic,
    path::Path,
    process::{Command, Output, Stdio},
    thread,
};

use tempfile::TempDir;

pub(crate) struct Workspace {
    root: TempDir,
}

impl Workspace {
    pub(crate) fn new() -> Self {
        let root = tempfile::tempdir().unwrap();

        for directory in [".application", ".configuration", ".home"] {
            fs::create_dir(root.path().join(directory)).unwrap();
        }

        fs::write(root.path().join("breathers.toml"), []).unwrap();

        Self { root }
    }

    pub(crate) fn path(&self) -> &Path {
        self.root.path()
    }

    pub(crate) fn command(&self) -> Command {
        let mut command = Command::new(env!("CARGO_BIN_EXE_breathers"));

        command
            .current_dir(self.path())
            .env("APPDATA", self.path().join(".application"))
            .env("XDG_CONFIG_HOME", self.path().join(".configuration"))
            .env("HOME", self.path().join(".home"))
            .env("NO_COLOR", "1");

        command
    }

    pub(crate) fn input(&self, arguments: &[&str], source: &[u8]) -> Output {
        let mut command = self.command();
        command.args(arguments);

        Self::execute(command, source).unwrap()
    }

    pub(crate) fn execute(mut command: Command, source: &[u8]) -> io::Result<Output> {
        let mut child = command
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?;

        let mut input = child
            .stdin
            .take()
            .ok_or_else(|| io::Error::other("Missing stdin pipe"))?;

        let source = source.to_owned();
        let writer = thread::spawn(move || input.write_all(&source));
        let output = child.wait_with_output()?;

        let written = match writer.join() {
            Ok(written) => written,
            Err(error) => panic::resume_unwind(error),
        };

        if output.status.success() {
            written?;
        }

        Ok(output)
    }
}
