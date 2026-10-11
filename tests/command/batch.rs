use std::fs;

use crate::support::Workspace;

#[test]
fn reports_batch_progress_and_summary() {
    let workspace = Workspace::new();
    let root = workspace.path();
    let source = r"local value = 1
return value
";
    let expected = r"local value = 1

return value
";
    fs::write(root.join("changed.luau"), source).unwrap();
    fs::write(root.join("unchanged.lua"), expected).unwrap();
    fs::write(root.join("broken.rs"), "fn broken( {").unwrap();

    let output = workspace
        .command()
        .env("CLICOLOR_FORCE", "1")
        .output()
        .unwrap();

    assert!(!output.status.success());
    assert_eq!(output.stdout, Vec::<u8>::new());
    let feedback = String::from_utf8(output.stderr).unwrap();
    assert!(feedback.starts_with("Scanning files...\n"));
    assert!(feedback.contains("Checking 1/3  broken.rs"));
    assert!(feedback.contains("Checking 3/3  unchanged.lua"));
    assert!(feedback.contains("Unchanged  unchanged.lua"));
    assert_eq!(feedback.matches("error: broken.rs (skipped)").count(), 1);
    assert!(feedback.find("error:").unwrap() < feedback.find("Checking 2/3").unwrap());

    assert!(
        feedback.find("Writing 1/1").unwrap() < feedback.find("Formatted  changed.luau").unwrap()
    );

    assert!(feedback.contains("3 processed · 1 changed · 1 unchanged · 1 failed · "));
    assert!(feedback.contains(" elapsed"));
    assert!(!feedback.contains('\u{1b}'));
    assert!(!feedback.contains('\r'));

    assert_eq!(
        fs::read_to_string(root.join("changed.luau")).unwrap(),
        expected
    );

    assert_eq!(
        fs::read_to_string(root.join("broken.rs")).unwrap(),
        "fn broken( {"
    );

    fs::remove_file(root.join("broken.rs")).unwrap();

    let output = workspace
        .command()
        .env("NO_COLOR", "1")
        .env("CLICOLOR_FORCE", "1")
        .output()
        .unwrap();

    assert!(output.status.success());
    let feedback = String::from_utf8(output.stderr).unwrap();
    assert!(feedback.contains("2 processed · 0 changed · 2 unchanged · 0 failed · "));
    assert!(!feedback.contains('\u{1b}'));
    assert!(!feedback.contains("Formatted"));
}

#[cfg(windows)]
#[test]
fn reports_partial_writes_accurately() {
    let workspace = Workspace::new();
    let root = workspace.path();
    let source = r"local value = 1
return value
";

    for name in ["first.luau", "second.luau", "third.luau"] {
        fs::write(root.join(name), source).unwrap();
    }

    let path = root.join("second.luau");
    let original = fs::metadata(&path).unwrap().permissions();
    let mut permissions = original.clone();
    permissions.set_readonly(true);
    fs::set_permissions(&path, permissions).unwrap();

    let output = workspace.command().output().unwrap();

    fs::set_permissions(&path, original).unwrap();
    assert!(!output.status.success());
    let feedback = String::from_utf8(output.stderr).unwrap();
    assert!(feedback.contains("Formatted  first.luau"));
    assert!(!feedback.contains("Formatted  second.luau"));
    assert!(!feedback.contains("Formatted  third.luau"));
    assert!(feedback.contains("error: second.luau"));
    assert!(feedback.contains("3 processed · 1 changed · 0 unchanged · 1 failed · "));
    assert!(feedback.contains("1 prepared changes not written"));

    assert_eq!(
        fs::read_to_string(root.join("first.luau")).unwrap(),
        r"local value = 1

return value
"
    );

    assert_eq!(fs::read_to_string(&path).unwrap(), source);
    assert_eq!(fs::read_to_string(root.join("third.luau")).unwrap(), source);
}

#[test]
fn reports_empty_selection_and_unwritten_changes() {
    let workspace = Workspace::new();
    let root = workspace.path();
    fs::write(root.join("breathers.toml"), "include = []\n").unwrap();
    let source = r"local value = 1
return value
";
    fs::write(root.join("changed.luau"), source).unwrap();
    fs::write(root.join("unknown"), source).unwrap();

    let output = workspace.command().output().unwrap();

    assert!(output.status.success());
    assert_eq!(output.stdout, Vec::<u8>::new());

    assert!(
        String::from_utf8_lossy(&output.stderr)
            .contains("0 processed · 0 changed · 0 unchanged · 0 failed · ")
    );

    fs::write(root.join("breathers.toml"), "").unwrap();

    let output = workspace
        .command()
        .args(["changed.luau", "unknown"])
        .output()
        .unwrap();

    assert!(!output.status.success());
    assert_eq!(output.stdout, Vec::<u8>::new());
    let feedback = String::from_utf8(output.stderr).unwrap();
    assert!(feedback.contains("error: unknown\n  Unknown language"));
    assert!(feedback.contains("Aborted"));
    assert!(feedback.contains("2 processed · 0 changed · 0 unchanged · 1 failed · "));
    assert!(feedback.contains("1 prepared changes not written"));
    assert!(!feedback.contains("Formatted"));

    assert_eq!(
        fs::read_to_string(root.join("changed.luau")).unwrap(),
        source
    );

    let output = workspace.command().arg("--schema").output().unwrap();

    assert!(output.status.success());
    assert_eq!(output.stderr, Vec::<u8>::new());
    serde_json::from_slice::<serde_json::Value>(&output.stdout).unwrap();
}

#[test]
fn formats_valid_files_and_reports_syntax_errors() {
    let workspace = Workspace::new();
    let root = workspace.path();
    let source = include_str!("../fixtures/c/command/standard/input.c");
    let expected = include_str!("../fixtures/c/command/standard/output.c");

    for (name, invalid, formatted) in [
        ("unknown", source, source),
        ("broken.cpp", "int broken( {", expected),
    ] {
        fs::write(root.join("main.c"), source).unwrap();
        fs::write(root.join(name), invalid).unwrap();

        let output = workspace.command().args(["main.c", name]).output().unwrap();

        assert!(!output.status.success());
        assert!(String::from_utf8_lossy(&output.stderr).contains(name));
        assert_eq!(fs::read_to_string(root.join("main.c")).unwrap(), formatted);
        assert_eq!(fs::read_to_string(root.join(name)).unwrap(), invalid);
    }

    fs::write(root.join("main.c"), source).unwrap();
    fs::write(root.join("broken.rs"), "fn broken( {").unwrap();

    let output = workspace
        .command()
        .args(["broken.cpp", "main.c", "broken.rs"])
        .output()
        .unwrap();

    assert!(!output.status.success());
    let errors = String::from_utf8_lossy(&output.stderr);
    assert!(errors.contains("broken.cpp"));
    assert!(errors.contains("broken.rs"));
    assert_eq!(fs::read_to_string(root.join("main.c")).unwrap(), expected);

    assert_eq!(
        fs::read_to_string(root.join("broken.cpp")).unwrap(),
        "int broken( {"
    );

    assert_eq!(
        fs::read_to_string(root.join("broken.rs")).unwrap(),
        "fn broken( {"
    );

    let source = include_str!("../fixtures/cplusplus/command/template/input.cpp");
    fs::write(root.join("cplusplus.h"), source).unwrap();

    let output = workspace.command().arg("cplusplus.h").output().unwrap();

    assert!(!output.status.success());

    let output = workspace
        .command()
        .args(["-l", "c++", "cplusplus.h"])
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );

    assert_eq!(
        fs::read_to_string(root.join("cplusplus.h")).unwrap(),
        include_str!("../fixtures/cplusplus/command/template/output.cpp")
    );
}
