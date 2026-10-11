use std::fs;

use crate::support::{Workspace, formatting, notification, open, protocol, request};

#[test]
fn formats_cplusplus_with_clangd_flags_and_buffer_contents() {
    use serde_json::json;
    use tower_lsp_server::ls_types::Uri;

    let workspace = Workspace::new();
    let root = workspace.path();
    fs::create_dir_all(root.join("bridge")).unwrap();
    fs::create_dir(root.join("include")).unwrap();

    fs::write(
        root.join(".clangd"),
        r"If:
  PathMatch: bridge/.*
CompileFlags:
  Add: [-std=c++17, -I../include]
",
    )
    .unwrap();

    fs::write(
        root.join("bridge/.clangd"),
        r"CompileFlags:
  Add: -DENABLED
",
    )
    .unwrap();

    fs::write(
        root.join("include/value.hpp"),
        r"#ifndef ENABLED
#error missing compiler flag
#endif
#define CHANGE(value) do { if (value) { ++value; } } while (0)
struct Value { Value() = default; };
",
    )
    .unwrap();

    let source = include_str!("../fixtures/cplusplus/command/compilation/input.cpp");
    let expected = include_str!("../fixtures/cplusplus/command/compilation/output.cpp");
    let path = root.join("bridge/source.cpp");
    fs::write(&path, source).unwrap();
    let uri = Uri::from_file_path(&path).unwrap();
    let buffer = source.replace("first", "second");

    let responses = protocol(
        &workspace,
        &["--lsp"],
        &[
            request(1, "initialize", json!({"capabilities": {}})),
            notification("initialized", json!({})),
            open(&uri, "cpp", &buffer),
            formatting(2, &uri),
            json!({"jsonrpc":"2.0","id":3,"method":"shutdown"}),
            json!({"jsonrpc":"2.0","method":"exit"}),
        ],
    );

    assert_eq!(
        responses[&2]["result"][0]["newText"],
        include_str!("../fixtures/cplusplus/command/compilation/buffer.cpp"),
        "{}",
        responses[&2]
    );

    assert_eq!(fs::read_to_string(&path).unwrap(), source);

    for _ in 0..2 {
        let output = workspace.input(&["bridge/source.cpp"], b"");

        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );

        assert_eq!(fs::read_to_string(&path).unwrap(), expected);
    }

    fs::write(root.join("bridge/.clangd"), "CompileFlags: [broken").unwrap();
    let output = workspace.input(&["bridge/source.cpp"], b"");
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains(".clangd"));
    assert_eq!(fs::read_to_string(&path).unwrap(), expected);
}

#[test]
fn formats_c_with_compilation_database_flags() {
    use serde_json::json;

    let workspace = Workspace::new();
    let root = workspace.path();
    fs::create_dir_all(root.join("build folder/include")).unwrap();
    fs::create_dir(root.join("nested")).unwrap();

    fs::write(
        root.join(".clangd"),
        r"CompileFlags:
  Add: [-UENABLED, -DENABLED=3]
",
    )
    .unwrap();

    fs::write(
        root.join("build folder/include/value.h"),
        r"#if ENABLED != 3
#error missing override
#endif
#define VALUE 1
",
    )
    .unwrap();

    let path = root.join("nested/source.c");

    let source = include_str!("../fixtures/c/command/database/input.c");
    let expected = include_str!("../fixtures/c/command/database/output.c");

    let mut entries = vec![
        json!({
            "directory": root.join("build folder"),
            "file": "../nested/source.c",
            "arguments": [
                "clang", "-Iinclude", "-DENABLED=2", "-c",
                "../nested/source.c", "-o", "ignored.o",
            ],
        }),
        json!({
            "directory": root.join("build folder"),
            "file": path,
            "command": r#"clang -I"include" -DENABLED=2 -c "../nested/source.c" -o ignored.o"#,
        }),
    ];

    if cfg!(windows) {
        entries.push(json!({
            "directory": root.join("build folder"),
            "file": path,
            "command": r#"cl.exe /nologo /TC /I"include" /DENABLED=2 /c "../nested/source.c" /Foignored.obj"#,
        }));
    }

    for entry in entries {
        fs::write(
            root.join("compile_commands.json"),
            json!([entry]).to_string(),
        )
        .unwrap();

        fs::write(&path, source).unwrap();
        let output = workspace.input(&["nested/source.c"], b"");

        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );

        assert_eq!(fs::read_to_string(&path).unwrap(), expected);
        assert!(!root.join("build folder/ignored.o").exists());
        assert!(!root.join("build folder/ignored.obj").exists());
    }

    fs::write(root.join("compile_commands.json"), "[").unwrap();
    let output = workspace.input(&["nested/source.c"], b"");
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("compile_commands.json"));
    assert_eq!(fs::read_to_string(&path).unwrap(), expected);
}

#[test]
fn formats_c_preprocessing_and_rejects_active_errors() {
    let workspace = Workspace::new();
    let root = workspace.path();
    fs::create_dir(root.join("include")).unwrap();

    fs::write(
        root.join(".clangd"),
        r"CompileFlags:
  Add: [-std=c11, -Iinclude, -DENABLED=1]
",
    )
    .unwrap();

    let header = include_str!("../fixtures/c/command/preprocessing/header.h");
    fs::write(root.join("include/value.h"), header).unwrap();

    let source = include_str!("../fixtures/c/command/preprocessing/input.c");
    let expected = include_str!("../fixtures/c/command/preprocessing/output.c");
    let path = root.join("source.c");
    fs::write(&path, source).unwrap();

    for _ in 0..2 {
        let output = workspace.input(&["source.c"], b"");

        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );

        assert_eq!(fs::read_to_string(&path).unwrap(), expected);
    }

    let output = workspace.input(&["-l", "c", "-"], source.as_bytes());

    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );

    assert_eq!(output.stdout, expected.as_bytes());

    let invalid = source.replace("return class;", "return class +;");
    fs::write(&path, &invalid).unwrap();
    let output = workspace.input(&["source.c"], b"");
    assert!(!output.status.success());
    assert_eq!(fs::read_to_string(&path).unwrap(), invalid);

    fs::write(
        root.join(".clangd"),
        r"CompileFlags:
  Add: -std=c11
",
    )
    .unwrap();

    let output = workspace.input(&["-l", "c", "-"], source.as_bytes());
    assert!(!output.status.success());
    assert_eq!(output.stdout, Vec::<u8>::new());

    assert_eq!(
        fs::read_to_string(root.join("include/value.h")).unwrap(),
        header
    );
}
