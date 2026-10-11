use std::fs;

use crate::support::Workspace;

const SOURCES: [(&str, &str, &str); 6] = [
    (
        "example.py",
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
        "example.nu",
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
        "example.js",
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
        "example.jsx",
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
    (
        "example.ts",
        r"function example(): number {
    work();
    return <number>result;
}
",
        r"function example(): number {
    work();

    return <number>result;
}
",
    ),
    (
        "example.tsx",
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
];

#[test]
fn discovers_new_languages_and_applies_project_configuration() {
    let workspace = workspace_with_sources();
    let root = workspace.path();

    let output = workspace.command().output().unwrap();

    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );

    for (name, source, formatted) in SOURCES {
        let expected = if matches!(name, "example.js" | "example.jsx") {
            formatted
        } else {
            source
        };

        assert_eq!(fs::read_to_string(root.join(name)).unwrap(), expected);
    }
}

#[test]
fn discovers_new_languages_and_applies_explicit_configuration() {
    let workspace = workspace_with_sources();
    let root = workspace.path();

    fs::write(
        root.join("settings.toml"),
        r"[javascript]
returns = false
",
    )
    .unwrap();

    let output = workspace
        .command()
        .args(["-c", "settings.toml"])
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );

    for (name, source, formatted) in SOURCES {
        let expected = if matches!(name, "example.js" | "example.jsx") {
            source
        } else {
            formatted
        };

        assert_eq!(fs::read_to_string(root.join(name)).unwrap(), expected);
    }
}

#[test]
fn rejects_invalid_configuration_without_writing_files_and_serves_schema() {
    let workspace = workspace_with_sources();
    let root = workspace.path();

    for invalid in [
        "[unknown]\nreturns = false",
        "[python]\nreturns = 3",
        "[javascript]\nunknown = false",
        "[nushell]\nclasses = false",
        "include = [\"[\"]",
        "exclude = [\"[\"]",
    ] {
        fs::write(root.join("breathers.toml"), invalid).unwrap();

        let output = workspace.command().output().unwrap();

        assert!(!output.status.success());

        for (name, source, _) in SOURCES {
            assert_eq!(fs::read_to_string(root.join(name)).unwrap(), source);
        }
    }

    let output = workspace
        .command()
        .args(["--config", "missing.toml"])
        .output()
        .unwrap();

    assert!(!output.status.success());

    let output = workspace.command().arg("--schema").output().unwrap();

    assert!(output.status.success());
    let schema: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();

    assert_eq!(
        schema,
        serde_json::from_str::<serde_json::Value>(include_str!("../../schema.json")).unwrap()
    );

    for (name, source, _) in SOURCES {
        assert_eq!(fs::read_to_string(root.join(name)).unwrap(), source);
    }
}

#[test]
fn detects_lua_and_luau_and_accepts_overrides() {
    let workspace = Workspace::new();
    let root = workspace.path();
    fs::create_dir(root.join("src")).unwrap();
    let source = r"local value = 1
return value
";
    let expected = r"local value = 1

return value
";
    fs::write(root.join("src/main.luau"), source).unwrap();
    fs::write(root.join("src/module.lua"), source).unwrap();

    let output = workspace.command().output().unwrap();

    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );

    assert_eq!(
        fs::read_to_string(root.join("src/main.luau")).unwrap(),
        expected
    );

    assert_eq!(
        fs::read_to_string(root.join("src/module.lua")).unwrap(),
        expected
    );

    let typed = r"local value: number = 1
return value
";
    let typed_expected = r"local value: number = 1

return value
";
    fs::write(root.join("src/main.luau"), source).unwrap();
    fs::write(root.join("src/module.lua"), typed).unwrap();

    let output = workspace.command().arg("src").output().unwrap();

    assert!(!output.status.success());

    assert_eq!(
        fs::read_to_string(root.join("src/main.luau")).unwrap(),
        expected
    );

    assert_eq!(
        fs::read_to_string(root.join("src/module.lua")).unwrap(),
        typed
    );

    for flag in ["-l", "--language"] {
        let output = workspace
            .command()
            .args([flag, "luau", "src"])
            .output()
            .unwrap();

        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );

        assert_eq!(
            fs::read_to_string(root.join("src/main.luau")).unwrap(),
            expected
        );

        assert_eq!(
            fs::read_to_string(root.join("src/module.lua")).unwrap(),
            typed_expected
        );
    }

    fs::write(root.join("src/main.luau"), source).unwrap();
    fs::write(root.join("src/module.lua"), "local broken = {").unwrap();

    let output = workspace.command().args(["-l", "luau"]).output().unwrap();

    assert!(!output.status.success());

    assert_eq!(
        fs::read_to_string(root.join("src/main.luau")).unwrap(),
        expected
    );

    assert_eq!(
        fs::read_to_string(root.join("src/module.lua")).unwrap(),
        "local broken = {"
    );
}

#[test]
fn detects_powershell_extensions_recursively() {
    let workspace = Workspace::new();
    let root = workspace.path();
    let nested = root.join("nested");
    fs::create_dir(&nested).unwrap();
    let source = r#"function example {
    Write-Output "ready"
    return 1
}
"#;
    let expected = r#"function example {
    Write-Output "ready"

    return 1
}
"#;

    for name in ["example.ps1", "module.psm1", "manifest.psd1"] {
        fs::write(nested.join(name), source).unwrap();
    }

    let output = workspace.command().output().unwrap();

    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );

    for name in ["example.ps1", "module.psm1", "manifest.psd1"] {
        assert_eq!(fs::read_to_string(nested.join(name)).unwrap(), expected);
    }
}

fn workspace_with_sources() -> Workspace {
    let workspace = Workspace::new();
    let root = workspace.path();

    for (name, source, _) in SOURCES {
        fs::write(root.join(name), source).unwrap();
    }

    fs::write(
        root.join("breathers.toml"),
        r"[python]
returns = false
[typescript]
returns = false
[nushell]
returns = false
",
    )
    .unwrap();

    workspace
}
