use std::fs;

use crate::support::Workspace;

#[test]
fn formats_rust_lua_and_c_standard_input_without_writing_files() {
    for (language, source, expected) in [
        (
            "rust",
            r"fn example() {
    work();
    return 1;
}
",
            r"fn example() {
    work();

    return 1;
}
",
        ),
        (
            "lua",
            r"local value = 1
return value
",
            r"local value = 1

return value
",
        ),
        (
            "luau",
            r"local value: number = 1
return value
",
            r"local value: number = 1

return value
",
        ),
        (
            "c",
            include_str!("../fixtures/c/command/standard/input.c"),
            include_str!("../fixtures/c/command/standard/output.c"),
        ),
        (
            "c++",
            r"int example() {
    work();
    return 1;
}
",
            r"int example() {
    work();

    return 1;
}
",
        ),
    ] {
        assert_standard_input(language, source, expected);
    }
}

#[test]
fn formats_python_and_shell_standard_input_without_writing_files() {
    for (language, source, expected) in [
        (
            "python",
            r"def example():
    work()
    return 1
",
            r"def example():
    work()

    return 1
",
        ),
        (
            "nushell",
            r"def example [] {
    print ready
    return 1
}
",
            r"def example [] {
    print ready

    return 1
}
",
        ),
        (
            "powershell",
            r#"function example {
    Write-Output "ready"
    return 1
}
"#,
            r#"function example {
    Write-Output "ready"

    return 1
}
"#,
        ),
    ] {
        assert_standard_input(language, source, expected);
    }
}

#[test]
fn formats_javascript_and_typescript_standard_input_without_writing_files() {
    for (language, source, expected) in [
        (
            "javascript",
            r"function example() {
    work();
    return 1;
}
",
            r"function example() {
    work();

    return 1;
}
",
        ),
        (
            "typescript",
            r"function example(): number {
    work();
    return 1;
}
",
            r"function example(): number {
    work();

    return 1;
}
",
        ),
        (
            "tsx",
            r"function example() {
    work();
    return <div />;
}
",
            r"function example() {
    work();

    return <div />;
}
",
        ),
    ] {
        assert_standard_input(language, source, expected);
    }
}

#[test]
fn accepts_empty_standard_input_without_writing_files() {
    let workspace = Workspace::new();
    let root = workspace.path();
    let untouched = r"local value = 1
return value
";
    fs::write(root.join("untouched.luau"), untouched).unwrap();

    fs::write(
        root.join("breathers.toml"),
        r#"include = []
exclude = ["**"]
"#,
    )
    .unwrap();

    let output = workspace.input(&["-l", "luau", "-"], b"");
    assert!(output.status.success());
    assert_eq!(output.stdout, Vec::<u8>::new());

    assert_eq!(
        fs::read_to_string(root.join("untouched.luau")).unwrap(),
        untouched
    );
}

#[test]
fn applies_standard_input_configuration_without_writing_files() {
    let workspace = Workspace::new();
    let root = workspace.path();
    let untouched = r"local value = 1
return value
";
    fs::write(root.join("untouched.luau"), untouched).unwrap();

    fs::write(
        root.join("breathers.toml"),
        r#"include = []
exclude = ["**"]
"#,
    )
    .unwrap();

    fs::write(
        root.join("settings.toml"),
        r"[luau]
returns = false
",
    )
    .unwrap();

    let output = workspace.input(
        &["--language", "luau", "--config", "settings.toml", "-"],
        untouched.as_bytes(),
    );

    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );

    assert_eq!(output.stdout, untouched.as_bytes());

    fs::write(
        root.join("breathers.toml"),
        r"[luau]
related = true
",
    )
    .unwrap();

    let output = workspace.input(&["-l", "luau", "-"], untouched.as_bytes());
    assert!(output.status.success());
    assert_eq!(output.stdout, untouched.as_bytes());

    assert_eq!(
        fs::read_to_string(root.join("untouched.luau")).unwrap(),
        untouched
    );
}

#[test]
fn rejects_invalid_standard_input_without_output_or_file_changes() {
    let workspace = Workspace::new();
    let root = workspace.path();
    let source = r"local value = 1
return value
";
    fs::write(root.join("untouched.luau"), source).unwrap();

    for (arguments, message) in [
        (vec!["-"], "Stdin requires --language"),
        (
            vec!["-l", "luau", "-", "untouched.luau"],
            "must be the only input path",
        ),
        (vec!["-l", "luau", "-", "-"], "must be the only input path"),
    ] {
        let output = workspace.command().args(arguments).output().unwrap();

        assert!(!output.status.success());
        assert_eq!(output.stdout, Vec::<u8>::new());
        assert!(String::from_utf8_lossy(&output.stderr).contains(message));
    }

    for source in [b"local broken = {".as_slice(), &[0xff]] {
        let output = workspace.input(&["-l", "luau", "-"], source);
        assert!(!output.status.success());
        assert_eq!(output.stdout, Vec::<u8>::new());
        assert!(String::from_utf8_lossy(&output.stderr).contains("stdin:"));
    }

    for configuration in [
        r"[luau]
unknown = true
",
        "include = [\"[\"]\n",
    ] {
        fs::write(root.join("breathers.toml"), configuration).unwrap();

        let output = workspace
            .command()
            .args(["-l", "luau", "-"])
            .output()
            .unwrap();

        assert!(!output.status.success());
        assert_eq!(output.stdout, Vec::<u8>::new());
    }

    assert_eq!(
        fs::read_to_string(root.join("untouched.luau")).unwrap(),
        source
    );
}

#[test]
fn applies_global_configuration_and_project_overrides_to_standard_input() {
    let workspace = Workspace::new();
    let root = workspace.path();

    let global = if cfg!(windows) {
        root.join(".application/breathers")
    } else if cfg!(target_os = "macos") {
        root.join(".home/Library/Application Support/breathers")
    } else {
        root.join(".configuration/breathers")
    };

    fs::create_dir_all(&global).unwrap();
    fs::write(global.join("config.toml"), "[luau]\nreturns = false\n").unwrap();

    let source = r"local value = 1
return value
";
    let output = workspace.input(&["-l", "luau", "-"], source.as_bytes());

    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );

    assert_eq!(output.stdout, source.as_bytes());
    assert_eq!(output.stderr, Vec::<u8>::new());

    fs::write(root.join("breathers.toml"), "[luau]\nreturns = true\n").unwrap();
    let output = workspace.input(&["-l", "luau", "-"], source.as_bytes());

    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );

    assert_eq!(output.stdout, b"local value = 1\n\nreturn value\n");
    assert_eq!(output.stderr, Vec::<u8>::new());
}

fn assert_standard_input(language: &str, source: &str, expected: &str) {
    let workspace = Workspace::new();
    let root = workspace.path();
    let untouched = r"local value = 1
return value
";
    fs::write(root.join("untouched.luau"), untouched).unwrap();

    fs::write(
        root.join("breathers.toml"),
        r#"include = []
exclude = ["**"]
"#,
    )
    .unwrap();

    for (source, expected) in [
        (source.to_owned(), expected.to_owned()),
        (source.replace('\n', "\r\n"), expected.replace('\n', "\r\n")),
        (
            source.trim_end_matches('\n').to_owned(),
            expected.trim_end_matches('\n').to_owned(),
        ),
    ] {
        let output = workspace.input(&["-l", language, "-"], source.as_bytes());

        assert!(
            output.status.success(),
            "{language}: {}",
            String::from_utf8_lossy(&output.stderr)
        );

        assert_eq!(output.stdout, expected.as_bytes(), "{language}");
        assert_eq!(output.stderr, Vec::<u8>::new());
    }

    assert_eq!(
        fs::read_to_string(root.join("untouched.luau")).unwrap(),
        untouched
    );
}
