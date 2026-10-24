//! Fixture-driven command integration tests.

mod support;

use std::{collections::BTreeMap, fs, path::Path};

use datatest_stable::{Result, harness};
use serde::Deserialize;

use support::Workspace;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Formatting {
    language: String,
    input: String,
    output: String,
    configuration: Option<String>,
    success: bool,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Scenario {
    files: BTreeMap<String, Option<String>>,
    steps: Vec<Step>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Step {
    arguments: Vec<String>,
    directory: Option<String>,
    input: Option<String>,
    success: bool,
    stdout: Option<String>,
    stderr: Option<String>,
    files: BTreeMap<String, String>,
    readonly: Option<String>,
}

#[derive(Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
struct Feedback {
    contains: Vec<String>,
    excludes: Vec<String>,
    ordered: Vec<String>,
}

fn formatting(path: &Path) -> Result<()> {
    let directory = path.parent().ok_or("Fixture has no directory")?;
    let cases: Vec<Formatting> = serde_json::from_str(&fs::read_to_string(path)?)?;

    for case in cases {
        let workspace = Workspace::new();

        if let Some(configuration) = case.configuration {
            fs::copy(
                directory.join(configuration),
                workspace.path().join("breathers.toml"),
            )?;
        }

        let source = fs::read_to_string(directory.join(&case.input))?;
        let expected = fs::read_to_string(directory.join(&case.output))?;

        for (source, expected) in [
            (source.clone(), expected.clone()),
            (source.replace('\n', "\r\n"), expected.replace('\n', "\r\n")),
        ] {
            let output = workspace.input(&["--language", &case.language, "-"], source.as_bytes());

            assert_eq!(
                output.status.success(),
                case.success,
                "{}",
                String::from_utf8_lossy(&output.stderr)
            );

            assert_eq!(output.stdout, expected.as_bytes(), "{}", path.display());

            let destination = workspace.path().join(
                Path::new(&case.input)
                    .file_name()
                    .ok_or("Fixture has no filename")?,
            );

            fs::write(&destination, &source)?;

            let output = workspace
                .command()
                .args(["--language", &case.language])
                .arg(&destination)
                .output()?;

            assert_eq!(
                output.status.success(),
                case.success,
                "{}",
                String::from_utf8_lossy(&output.stderr)
            );

            assert_eq!(
                fs::read_to_string(&destination)?.as_str(),
                if case.success {
                    expected.as_str()
                } else {
                    source.as_str()
                }
            );

            if case.success {
                let output =
                    workspace.input(&["--language", &case.language, "-"], expected.as_bytes());

                assert!(
                    output.status.success(),
                    "{}",
                    String::from_utf8_lossy(&output.stderr)
                );

                assert_eq!(output.stdout, expected.as_bytes());
            }
        }
    }

    Ok(())
}

fn commands(path: &Path) -> Result<()> {
    let directory = path.parent().ok_or("Fixture has no directory")?;
    let scenario: Scenario = serde_json::from_str(&fs::read_to_string(path)?)?;
    let workspace = Workspace::new();

    for (destination, source) in scenario.files {
        let destination = workspace.path().join(destination);

        if let Some(source) = source {
            fs::create_dir_all(destination.parent().ok_or("Fixture has no parent")?)?;
            fs::copy(directory.join(source), destination)?;
        } else {
            fs::remove_file(destination)?;
        }
    }

    for step in scenario.steps {
        let readonly = step.readonly.map(|path| workspace.path().join(path));

        let permissions = readonly
            .as_ref()
            .map(|path| fs::metadata(path).map(|metadata| metadata.permissions()))
            .transpose()?;

        if let (Some(path), Some(permissions)) = (&readonly, &permissions) {
            let mut permissions = permissions.clone();
            permissions.set_readonly(true);
            fs::set_permissions(path, permissions)?;
        }

        let mut command = workspace.command();
        command.args(step.arguments);

        if let Some(directory) = step.directory {
            command.current_dir(workspace.path().join(directory));
        }

        let output = Workspace::execute(
            command,
            &step
                .input
                .map(|input| fs::read(directory.join(input)))
                .transpose()?
                .unwrap_or_default(),
        );

        if let (Some(path), Some(permissions)) = (readonly, permissions) {
            fs::set_permissions(path, permissions)?;
        }

        let output = output?;

        assert_eq!(
            output.status.success(),
            step.success,
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );

        if let Some(expected) = step.stdout {
            assert_eq!(output.stdout, fs::read(directory.join(expected))?);
        }

        if let Some(expected) = step.stderr {
            let expected: Feedback =
                serde_json::from_str(&fs::read_to_string(directory.join(expected))?)?;

            feedback(&String::from_utf8(output.stderr)?, expected);
        }

        for (destination, expected) in step.files {
            assert_eq!(
                fs::read(workspace.path().join(destination))?,
                fs::read(directory.join(expected))?
            );
        }
    }

    Ok(())
}

fn feedback(actual: &str, expected: Feedback) {
    for message in expected.contains {
        assert!(actual.contains(&message), "{actual}");
    }

    for message in expected.excludes {
        assert!(!actual.contains(&message), "{actual}");
    }

    let mut remaining = actual;

    for message in expected.ordered {
        let position = remaining.find(&message).expect(actual);
        remaining = &remaining[position + message.len()..];
    }
}

harness! {
    { test = formatting, root = "tests/fixtures", pattern = r"(^|/)formatting\.json$" },
    { test = commands, root = "tests/fixtures", pattern = r"(^|/)command\.json$" },
}
