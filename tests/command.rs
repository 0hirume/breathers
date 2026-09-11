use std::{
    fs,
    io::Write,
    process::{Command, Output, Stdio},
};

fn directory() -> std::path::PathBuf {
    loop {
        let root = std::env::temp_dir().join(format!(
            "breathers-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));

        match fs::create_dir(&root) {
            Ok(()) => return root,
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {}
            Err(error) => panic!("{}: {error}", root.display()),
        }
    }
}

fn input(root: &std::path::Path, arguments: &[&str], source: &[u8]) -> Output {
    let mut child = Command::new(env!("CARGO_BIN_EXE_breathers"))
        .current_dir(root)
        .args(arguments)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();

    child.stdin.take().unwrap().write_all(source).unwrap();

    child.wait_with_output().unwrap()
}

fn protocol(
    root: &std::path::Path,
    arguments: &[&str],
    messages: &[serde_json::Value],
) -> std::collections::BTreeMap<i64, serde_json::Value> {
    use std::io::{BufRead, Read};

    let mut child = Command::new(env!("CARGO_BIN_EXE_breathers"))
        .current_dir(root)
        .args(arguments)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();

    let mut sender = child.stdin.take().unwrap();
    let mut receiver = std::io::BufReader::new(child.stdout.take().unwrap());
    let mut responses = std::collections::BTreeMap::new();

    for message in messages {
        let body = message.to_string();
        write!(sender, "Content-Length: {}\r\n\r\n{body}", body.len()).unwrap();
        sender.flush().unwrap();

        if let Some(identifier) = message["id"].as_i64() {
            let mut length = None;

            loop {
                let mut header = String::new();

                assert_ne!(
                    receiver.read_line(&mut header).unwrap(),
                    0,
                    "Server closed stdout"
                );

                if header == "\r\n" {
                    break;
                }

                if let Some(value) = header.strip_prefix("Content-Length: ") {
                    length = Some(value.trim().parse::<usize>().unwrap());
                }
            }

            let mut body = vec![0; length.unwrap()];
            receiver.read_exact(&mut body).unwrap();
            let response: serde_json::Value = serde_json::from_slice(&body).unwrap();
            assert_eq!(response["id"], identifier);
            responses.insert(identifier, response);
        }
    }

    drop(sender);
    let output = child.wait_with_output().unwrap();

    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );

    responses
}

#[test]
fn formats_cplusplus_with_clangd_flags_and_buffer_contents() {
    use serde_json::json;
    use tower_lsp_server::ls_types::Uri;

    let root = directory();
    fs::create_dir_all(root.join("bridge")).unwrap();
    fs::create_dir(root.join("include")).unwrap();
    fs::write(root.join("breathers.toml"), "").unwrap();

    fs::write(
        root.join(".clangd"),
        "If:\n  PathMatch: bridge/.*\nCompileFlags:\n  Add: [-std=c++17, -I../include]\n",
    )
    .unwrap();

    fs::write(
        root.join("bridge/.clangd"),
        "CompileFlags:\n  Add: -DENABLED\n",
    )
    .unwrap();

    fs::write(root.join("include/value.hpp"), "#ifndef ENABLED\n#error missing compiler flag\n#endif\n#define CHANGE(value) do { if (value) { ++value; } } while (0)\nstruct Value { Value() = default; };\n").unwrap();

    let source = "#include <value.hpp>\nint example(Value parent = {}) {\n    int first = 1;\n    CHANGE(first);\n    return first;\n}\n";
    let expected = source.replace("    return", "\n    return");
    let path = root.join("bridge/source.cpp");
    fs::write(&path, source).unwrap();
    let uri = Uri::from_file_path(&path).unwrap();
    let buffer = source.replace("first", "second");

    let responses = protocol(
        &root,
        &["--lsp"],
        &[
            json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{"capabilities":{}}}),
            json!({"jsonrpc":"2.0","method":"initialized","params":{}}),
            json!({"jsonrpc":"2.0","method":"textDocument/didOpen","params":{"textDocument":{"uri":uri,"languageId":"cpp","version":1,"text":buffer}}}),
            json!({"jsonrpc":"2.0","id":2,"method":"textDocument/formatting","params":{"textDocument":{"uri":uri},"options":{"tabSize":4,"insertSpaces":true}}}),
            json!({"jsonrpc":"2.0","id":3,"method":"shutdown"}),
            json!({"jsonrpc":"2.0","method":"exit"}),
        ],
    );

    assert_eq!(
        responses[&2]["result"][0]["newText"],
        expected.replace("first", "second"),
        "{}",
        responses[&2]
    );

    assert_eq!(fs::read_to_string(&path).unwrap(), source);

    for _ in 0..2 {
        let output = input(&root, &["bridge/source.cpp"], b"");

        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );

        assert_eq!(fs::read_to_string(&path).unwrap(), expected);
    }

    fs::write(root.join("bridge/.clangd"), "CompileFlags: [broken").unwrap();
    let output = input(&root, &["bridge/source.cpp"], b"");
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains(".clangd"));
    assert_eq!(fs::read_to_string(&path).unwrap(), expected);
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn serves_nushell_formatting() {
    use serde_json::json;

    let root = directory();
    fs::write(root.join("breathers.toml"), "").unwrap();
    let source = "print ready\nreturn done\n";

    for language in ["nu", "nushell"] {
        let responses = protocol(
            &root,
            &["--lsp"],
            &[
                json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{"capabilities":{}}}),
                json!({"jsonrpc":"2.0","method":"initialized","params":{}}),
                json!({"jsonrpc":"2.0","method":"textDocument/didOpen","params":{"textDocument":{"uri":"untitled:example","languageId":language,"version":1,"text":source}}}),
                json!({"jsonrpc":"2.0","id":2,"method":"textDocument/formatting","params":{"textDocument":{"uri":"untitled:example"},"options":{"tabSize":4,"insertSpaces":true}}}),
                json!({"jsonrpc":"2.0","method":"textDocument/didChange","params":{"textDocument":{"uri":"untitled:example","version":2},"contentChanges":[{"text":"def broken [] {"}]}}),
                json!({"jsonrpc":"2.0","id":3,"method":"textDocument/formatting","params":{"textDocument":{"uri":"untitled:example"},"options":{"tabSize":4,"insertSpaces":true}}}),
                json!({"jsonrpc":"2.0","id":4,"method":"shutdown"}),
                json!({"jsonrpc":"2.0","method":"exit"}),
            ],
        );

        assert_eq!(
            responses[&2]["result"][0]["newText"],
            "print ready\n\nreturn done\n"
        );

        assert!(responses[&3].get("error").is_some());
    }

    fs::remove_dir_all(root).unwrap();
}

#[test]
fn serves_document_formatting_with_buffer_synchronization() {
    use serde_json::json;
    use tower_lsp_server::ls_types::Uri;

    let root = directory();

    let project = root.join("project");
    fs::create_dir_all(project.join("src/generated")).unwrap();
    fs::write(root.join("breathers.toml"), "[luau]\nreturns = false\n").unwrap();

    fs::write(
        project.join("breathers.toml"),
        "include = [\"src/**\"]\nexclude = [\"**/generated/**\"]\n",
    )
    .unwrap();

    let disk = "local disk = 1\nreturn disk\n";
    fs::write(project.join("src/main.luau"), disk).unwrap();
    let uri = Uri::from_file_path(project.join("src/main.luau")).unwrap();
    let excluded = Uri::from_file_path(project.join("src/generated/example.luau")).unwrap();
    let source = "local value = 1\r\nreturn '🙂'";
    let formatted = "local value = 1\r\n\r\nreturn '🙂'";

    let spaced =
        "local first = require(\"shared/first\")\n\nlocal second = require(\"shared/second\")\n";

    let responses = protocol(
        &root,
        &["--lsp"],
        &[
            json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{"capabilities":{}}}),
            json!({"jsonrpc":"2.0","method":"initialized","params":{}}),
            json!({"jsonrpc":"2.0","method":"textDocument/didOpen","params":{"textDocument":{"uri":uri,"languageId":"luau","version":1,"text":source}}}),
            json!({"jsonrpc":"2.0","id":2,"method":"textDocument/formatting","params":{"textDocument":{"uri":uri},"options":{"tabSize":4,"insertSpaces":true}}}),
            json!({"jsonrpc":"2.0","method":"textDocument/didChange","params":{"textDocument":{"uri":uri,"version":2},"contentChanges":[{"text":formatted}]}}),
            json!({"jsonrpc":"2.0","method":"textDocument/didChange","params":{"textDocument":{"uri":uri,"version":1},"contentChanges":[{"text":"local broken = {"}]}}),
            json!({"jsonrpc":"2.0","id":3,"method":"textDocument/formatting","params":{"textDocument":{"uri":uri},"options":{"tabSize":4,"insertSpaces":true}}}),
            json!({"jsonrpc":"2.0","method":"textDocument/didChange","params":{"textDocument":{"uri":uri,"version":3},"contentChanges":[{"text":"local broken = {"}]}}),
            json!({"jsonrpc":"2.0","id":4,"method":"textDocument/formatting","params":{"textDocument":{"uri":uri},"options":{"tabSize":4,"insertSpaces":true}}}),
            json!({"jsonrpc":"2.0","method":"textDocument/didChange","params":{"textDocument":{"uri":uri,"version":4},"contentChanges":[{"range":{"start":{"line":0,"character":0},"end":{"line":0,"character":1}},"text":"x"}]}}),
            json!({"jsonrpc":"2.0","id":5,"method":"textDocument/formatting","params":{"textDocument":{"uri":uri},"options":{"tabSize":4,"insertSpaces":true}}}),
            json!({"jsonrpc":"2.0","method":"textDocument/didChange","params":{"textDocument":{"uri":uri,"version":5},"contentChanges":[{"text":spaced}]}}),
            json!({"jsonrpc":"2.0","id":6,"method":"textDocument/formatting","params":{"textDocument":{"uri":uri},"options":{"tabSize":4,"insertSpaces":true}}}),
            json!({"jsonrpc":"2.0","method":"textDocument/didClose","params":{"textDocument":{"uri":uri}}}),
            json!({"jsonrpc":"2.0","id":7,"method":"textDocument/formatting","params":{"textDocument":{"uri":uri},"options":{"tabSize":4,"insertSpaces":true}}}),
            json!({"jsonrpc":"2.0","method":"textDocument/didOpen","params":{"textDocument":{"uri":excluded,"languageId":"luau","version":1,"text":source}}}),
            json!({"jsonrpc":"2.0","id":8,"method":"textDocument/formatting","params":{"textDocument":{"uri":excluded},"options":{"tabSize":4,"insertSpaces":true}}}),
            json!({"jsonrpc":"2.0","method":"textDocument/didOpen","params":{"textDocument":{"uri":"untitled:example","languageId":"luau","version":1,"text":source}}}),
            json!({"jsonrpc":"2.0","id":9,"method":"textDocument/formatting","params":{"textDocument":{"uri":"untitled:example"},"options":{"tabSize":4,"insertSpaces":true}}}),
            json!({"jsonrpc":"2.0","id":10,"method":"shutdown"}),
            json!({"jsonrpc":"2.0","method":"exit"}),
        ],
    );

    assert_eq!(responses.len(), 10);
    assert!(responses[&1].get("error").is_none(), "{}", responses[&1]);

    assert_eq!(
        responses[&1]["result"]["capabilities"]["textDocumentSync"],
        1
    );

    assert_eq!(
        responses[&1]["result"]["capabilities"]["documentFormattingProvider"],
        true
    );

    assert_eq!(responses[&2]["result"][0]["newText"], formatted);

    assert_eq!(
        responses[&2]["result"][0]["range"]["end"],
        json!({"line":1,"character":11})
    );

    for identifier in [3, 6, 8, 9, 10] {
        assert!(
            responses[&identifier]["result"].is_null(),
            "{}",
            responses[&identifier]
        );

        assert!(
            responses[&identifier].get("error").is_none(),
            "{}",
            responses[&identifier]
        );
    }

    for identifier in [4, 5, 7] {
        assert_eq!(responses[&identifier]["error"]["code"], -32602);
    }

    assert_eq!(
        fs::read_to_string(project.join("src/main.luau")).unwrap(),
        disk
    );

    fs::remove_dir_all(root).unwrap();
}

#[test]
fn applies_explicit_server_configuration_and_rejects_mixed_modes() {
    use serde_json::json;

    let root = directory();

    fs::create_dir_all(root.join("nested")).unwrap();

    fs::write(
        root.join("nested/breathers.toml"),
        "[luau]\nreturns = false\n",
    )
    .unwrap();

    let uri =
        tower_lsp_server::ls_types::Uri::from_file_path(root.join("nested/example.luau")).unwrap();

    let source = "local value = 1\nreturn value\n";

    for (configuration, expected) in [
        ("[luau]\nrelated = true\n", None),
        (
            "[luau]\nrelated = false\n",
            Some("local value = 1\n\nreturn value\n"),
        ),
        ("include = []\n", None),
        ("[luau]\nunknown = true\n", None),
    ] {
        fs::write(root.join("selected.toml"), configuration).unwrap();

        let responses = protocol(
            &root,
            &["--lsp", "-c", "selected.toml"],
            &[
                json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{"capabilities":{}}}),
                json!({"jsonrpc":"2.0","method":"initialized","params":{}}),
                json!({"jsonrpc":"2.0","method":"textDocument/didOpen","params":{"textDocument":{"uri":uri,"languageId":"unknown","version":1,"text":source}}}),
                json!({"jsonrpc":"2.0","id":2,"method":"textDocument/formatting","params":{"textDocument":{"uri":uri},"options":{"tabSize":4,"insertSpaces":true}}}),
                json!({"jsonrpc":"2.0","id":3,"method":"shutdown"}),
                json!({"jsonrpc":"2.0","method":"exit"}),
            ],
        );

        if configuration.contains("unknown") {
            assert_eq!(responses[&2]["error"]["code"], -32602);
        } else {
            assert!(responses[&2].get("error").is_none(), "{}", responses[&2]);

            if let Some(expected) = expected {
                assert_eq!(responses[&2]["result"][0]["newText"], expected);
            } else {
                assert!(responses[&2]["result"].is_null());
            }
        }
    }

    for arguments in [
        vec!["--lsp", "-"],
        vec!["--lsp", "."],
        vec!["--lsp", "-l", "luau"],
        vec!["--lsp", "--schema"],
    ] {
        let output = Command::new(env!("CARGO_BIN_EXE_breathers"))
            .current_dir(&root)
            .args(arguments)
            .output()
            .unwrap();

        assert!(!output.status.success());
        assert_eq!(output.stdout, Vec::<u8>::new());
    }

    assert!(!root.join("nested/example.luau").exists());
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn formats_standard_input_without_writing_files() {
    let root = directory();
    let untouched = "local value = 1\nreturn value\n";
    fs::write(root.join("untouched.luau"), untouched).unwrap();

    fs::write(
        root.join("breathers.toml"),
        "include = []\nexclude = [\"**\"]\n",
    )
    .unwrap();

    for (language, source) in [
        ("rust", "fn example() {\n    work();\n    return 1;\n}\n"),
        ("lua", "local value = 1\nreturn value\n"),
        ("luau", "local value: number = 1\nreturn value\n"),
        ("c", "int example(void) {\n    work();\n    return 1;\n}\n"),
        ("c++", "int example() {\n    work();\n    return 1;\n}\n"),
        ("python", "def example():\n    work()\n    return 1\n"),
        (
            "nushell",
            "def example [] {\n    print ready\n    return 1\n}\n",
        ),
        (
            "javascript",
            "function example() {\n    work();\n    return 1;\n}\n",
        ),
        (
            "typescript",
            "function example(): number {\n    work();\n    return 1;\n}\n",
        ),
        (
            "tsx",
            "function example() {\n    work();\n    return <div />;\n}\n",
        ),
    ] {
        let expected = source
            .replace("\n    return", "\n\n    return")
            .replace("\nreturn", "\n\nreturn");

        for (source, expected) in [
            (source.to_owned(), expected.clone()),
            (source.replace('\n', "\r\n"), expected.replace('\n', "\r\n")),
            (
                source.trim_end_matches('\n').to_owned(),
                expected.trim_end_matches('\n').to_owned(),
            ),
        ] {
            let output = input(&root, &["-l", language, "-"], source.as_bytes());

            assert!(
                output.status.success(),
                "{language}: {}",
                String::from_utf8_lossy(&output.stderr)
            );

            assert_eq!(output.stdout, expected.as_bytes(), "{language}");
            assert_eq!(output.stderr, Vec::<u8>::new());
        }
    }

    let output = input(&root, &["-l", "luau", "-"], b"");
    assert!(output.status.success());
    assert_eq!(output.stdout, Vec::<u8>::new());
    fs::write(root.join("settings.toml"), "[luau]\nreturns = false\n").unwrap();

    let output = input(
        &root,
        &["--language", "luau", "--config", "settings.toml", "-"],
        untouched.as_bytes(),
    );

    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );

    assert_eq!(output.stdout, untouched.as_bytes());
    fs::write(root.join("breathers.toml"), "[luau]\nrelated = true\n").unwrap();
    let output = input(&root, &["-l", "luau", "-"], untouched.as_bytes());
    assert!(output.status.success());
    assert_eq!(output.stdout, untouched.as_bytes());

    assert_eq!(
        fs::read_to_string(root.join("untouched.luau")).unwrap(),
        untouched
    );

    fs::remove_dir_all(root).unwrap();
}

#[test]
fn rejects_invalid_standard_input_without_output_or_file_changes() {
    let root = directory();
    let source = "local value = 1\nreturn value\n";
    fs::write(root.join("untouched.luau"), source).unwrap();
    fs::write(root.join("breathers.toml"), "").unwrap();

    for (arguments, message) in [
        (vec!["-"], "Stdin requires --language"),
        (
            vec!["-l", "luau", "-", "untouched.luau"],
            "must be the only input path",
        ),
        (vec!["-l", "luau", "-", "-"], "must be the only input path"),
    ] {
        let output = Command::new(env!("CARGO_BIN_EXE_breathers"))
            .current_dir(&root)
            .args(arguments)
            .output()
            .unwrap();

        assert!(!output.status.success());
        assert_eq!(output.stdout, Vec::<u8>::new());
        assert!(String::from_utf8_lossy(&output.stderr).contains(message));
    }

    for source in [b"local broken = {".as_slice(), &[0xff]] {
        let output = input(&root, &["-l", "luau", "-"], source);
        assert!(!output.status.success());
        assert_eq!(output.stdout, Vec::<u8>::new());
        assert!(String::from_utf8_lossy(&output.stderr).contains("stdin:"));
    }

    for configuration in ["[luau]\nunknown = true\n", "include = [\"[\"]\n"] {
        fs::write(root.join("breathers.toml"), configuration).unwrap();

        let output = Command::new(env!("CARGO_BIN_EXE_breathers"))
            .current_dir(&root)
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

    fs::remove_dir_all(root).unwrap();
}

#[test]
fn discovers_new_languages_and_loads_configuration_before_writing() {
    let root = directory();

    let sources = [
        ("example.py", "def example():\n    work()\n    return 1\n"),
        (
            "example.nu",
            "def example [] {\n    print ready\n    return 1\n}\n",
        ),
        (
            "example.js",
            "function example() {\n    work();\n    return 1;\n}\n",
        ),
        (
            "example.jsx",
            "function example() {\n    work();\n    return <div />;\n}\n",
        ),
        (
            "example.ts",
            "function example(): number {\n    work();\n    return <number>result;\n}\n",
        ),
        (
            "example.tsx",
            "function example() {\n    work();\n    return <div />;\n}\n",
        ),
    ];

    for (name, source) in sources {
        fs::write(root.join(name), source).unwrap();
    }

    fs::write(
        root.join("breathers.toml"),
        "[python]\nreturns = false\n[typescript]\nreturns = false\n[nushell]\nreturns = false\n",
    )
    .unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_breathers"))
        .current_dir(&root)
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );

    for (name, source) in sources {
        let expected = if matches!(name, "example.js" | "example.jsx") {
            source.replace("    return", "\n    return")
        } else {
            source.to_owned()
        };

        assert_eq!(fs::read_to_string(root.join(name)).unwrap(), expected);
        fs::write(root.join(name), source).unwrap();
    }

    fs::write(
        root.join("settings.toml"),
        "[javascript]\nreturns = false\n",
    )
    .unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_breathers"))
        .current_dir(&root)
        .args(["-c", "settings.toml"])
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );

    for (name, source) in sources {
        let expected = if matches!(name, "example.js" | "example.jsx") {
            source.to_owned()
        } else {
            source.replace("    return", "\n    return")
        };

        assert_eq!(fs::read_to_string(root.join(name)).unwrap(), expected);
        fs::write(root.join(name), source).unwrap();
    }

    reject_invalid_configuration(&root, &sources);
    fs::remove_dir_all(root).unwrap();
}

fn reject_invalid_configuration(root: &std::path::Path, sources: &[(&str, &str)]) {
    for invalid in [
        "[unknown]\nreturns = false",
        "[python]\nreturns = 3",
        "[javascript]\nunknown = false",
        "[nushell]\nclasses = false",
        "include = [\"[\"]",
        "exclude = [\"[\"]",
    ] {
        fs::write(root.join("breathers.toml"), invalid).unwrap();

        let output = Command::new(env!("CARGO_BIN_EXE_breathers"))
            .current_dir(root)
            .output()
            .unwrap();

        assert!(!output.status.success());

        for &(name, source) in sources {
            assert_eq!(fs::read_to_string(root.join(name)).unwrap(), source);
        }
    }

    let output = Command::new(env!("CARGO_BIN_EXE_breathers"))
        .current_dir(root)
        .args(["--config", "missing.toml"])
        .output()
        .unwrap();

    assert!(!output.status.success());

    let output = Command::new(env!("CARGO_BIN_EXE_breathers"))
        .current_dir(root)
        .arg("--schema")
        .output()
        .unwrap();

    assert!(output.status.success());
    let schema: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();

    assert_eq!(
        schema,
        serde_json::from_str::<serde_json::Value>(include_str!("../schema.json")).unwrap()
    );

    for &(name, source) in sources {
        assert_eq!(fs::read_to_string(root.join(name)).unwrap(), source);
    }
}

#[test]
fn filters_discovered_and_explicit_files_from_the_project_root() {
    let root = directory();

    let sources = root.join("src");
    fs::create_dir_all(sources.join("generated")).unwrap();
    fs::write(root.join("breathers.toml"), "include = [\"src/**/*.luau\"]\nexclude = [\"**/generated/**\", \"src/omit.luau\"]\n[luau]\nrelated = true\n").unwrap();

    for file in ["generated/invalid.luau", "omit.luau", "other.lua"] {
        fs::write(sources.join(file), "local broken = {").unwrap();
    }

    let related = "local value = 1\nreturn value\n";
    let separate = "work()\nreturn 1\n";

    for (arguments, formatted) in [
        (vec![], true),
        (vec!["main.luau"], false),
        (
            vec![
                "standalone.luau",
                "generated/invalid.luau",
                "omit.luau",
                "other.lua",
            ],
            true,
        ),
    ] {
        fs::write(sources.join("main.luau"), related).unwrap();
        fs::write(sources.join("standalone.luau"), separate).unwrap();

        let output = Command::new(env!("CARGO_BIN_EXE_breathers"))
            .current_dir(&sources)
            .args(arguments)
            .output()
            .unwrap();

        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );

        assert_eq!(
            fs::read_to_string(sources.join("main.luau")).unwrap(),
            related
        );

        assert_eq!(
            fs::read_to_string(sources.join("standalone.luau")).unwrap(),
            if formatted {
                "work()\n\nreturn 1\n"
            } else {
                separate
            }
        );

        for file in ["generated/invalid.luau", "omit.luau", "other.lua"] {
            assert_eq!(
                fs::read_to_string(sources.join(file)).unwrap(),
                "local broken = {"
            );
        }
    }

    fs::remove_dir_all(root).unwrap();
}

#[test]
fn detects_lua_and_luau_and_accepts_overrides() {
    let root = directory();
    fs::create_dir(root.join("src")).unwrap();
    let source = "local value = 1\nreturn value\n";
    let expected = "local value = 1\n\nreturn value\n";
    fs::write(root.join("src/main.luau"), source).unwrap();
    fs::write(root.join("src/module.lua"), source).unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_breathers"))
        .current_dir(&root)
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
        expected
    );

    let typed = "local value: number = 1\nreturn value\n";
    let typed_expected = "local value: number = 1\n\nreturn value\n";
    fs::write(root.join("src/main.luau"), source).unwrap();
    fs::write(root.join("src/module.lua"), typed).unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_breathers"))
        .current_dir(&root)
        .arg("src")
        .output()
        .unwrap();

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
        let output = Command::new(env!("CARGO_BIN_EXE_breathers"))
            .current_dir(&root)
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

    let output = Command::new(env!("CARGO_BIN_EXE_breathers"))
        .current_dir(&root)
        .args(["-l", "luau"])
        .output()
        .unwrap();

    assert!(!output.status.success());

    assert_eq!(
        fs::read_to_string(root.join("src/main.luau")).unwrap(),
        expected
    );

    assert_eq!(
        fs::read_to_string(root.join("src/module.lua")).unwrap(),
        "local broken = {"
    );

    fs::remove_dir_all(root).unwrap();
}

#[test]
fn reports_batch_progress_and_summary() {
    let root = directory();
    fs::write(root.join("breathers.toml"), "").unwrap();
    let source = "local value = 1\nreturn value\n";
    let expected = "local value = 1\n\nreturn value\n";
    fs::write(root.join("changed.luau"), source).unwrap();
    fs::write(root.join("unchanged.lua"), expected).unwrap();
    fs::write(root.join("broken.rs"), "fn broken( {").unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_breathers"))
        .current_dir(&root)
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

    let output = Command::new(env!("CARGO_BIN_EXE_breathers"))
        .current_dir(&root)
        .env("NO_COLOR", "1")
        .env("CLICOLOR_FORCE", "1")
        .output()
        .unwrap();

    assert!(output.status.success());
    let feedback = String::from_utf8(output.stderr).unwrap();
    assert!(feedback.contains("2 processed · 0 changed · 2 unchanged · 0 failed · "));
    assert!(!feedback.contains('\u{1b}'));
    assert!(!feedback.contains("Formatted"));
    fs::remove_dir_all(root).unwrap();
}

#[cfg(windows)]
#[test]
fn reports_partial_writes_accurately() {
    let root = directory();
    fs::write(root.join("breathers.toml"), "").unwrap();
    let source = "local value = 1\nreturn value\n";

    for name in ["first.luau", "second.luau", "third.luau"] {
        fs::write(root.join(name), source).unwrap();
    }

    let path = root.join("second.luau");
    let original = fs::metadata(&path).unwrap().permissions();
    let mut permissions = original.clone();
    permissions.set_readonly(true);
    fs::set_permissions(&path, permissions).unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_breathers"))
        .current_dir(&root)
        .output()
        .unwrap();

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
        source.replace("\nreturn", "\n\nreturn")
    );

    assert_eq!(fs::read_to_string(&path).unwrap(), source);
    assert_eq!(fs::read_to_string(root.join("third.luau")).unwrap(), source);
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn reports_empty_selection_and_unwritten_changes() {
    let root = directory();
    fs::write(root.join("breathers.toml"), "include = []\n").unwrap();
    let source = "local value = 1\nreturn value\n";
    fs::write(root.join("changed.luau"), source).unwrap();
    fs::write(root.join("unknown"), source).unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_breathers"))
        .current_dir(&root)
        .output()
        .unwrap();

    assert!(output.status.success());
    assert_eq!(output.stdout, Vec::<u8>::new());

    assert!(
        String::from_utf8_lossy(&output.stderr)
            .contains("0 processed · 0 changed · 0 unchanged · 0 failed · ")
    );

    fs::write(root.join("breathers.toml"), "").unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_breathers"))
        .current_dir(&root)
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

    let output = Command::new(env!("CARGO_BIN_EXE_breathers"))
        .current_dir(&root)
        .arg("--schema")
        .output()
        .unwrap();

    assert!(output.status.success());
    assert_eq!(output.stderr, Vec::<u8>::new());
    assert!(serde_json::from_slice::<serde_json::Value>(&output.stdout).is_ok());
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn formats_selected_files() {
    let root = directory();
    fs::create_dir(root.join("src")).unwrap();
    fs::create_dir(root.join("target")).unwrap();
    let source = "int example(void) {\n    work();\n    return 1;\n}\n";
    let expected = source.replace("    return", "\n    return");

    let files = [
        "main.c",
        "src/main.cpp",
        "src/header.h",
        "src/header.hpp",
        "target/generated.c",
    ];

    for (arguments, changed) in [
        (vec![], files.to_vec()),
        (
            vec!["src"],
            vec!["src/main.cpp", "src/header.h", "src/header.hpp"],
        ),
        (vec!["src/main.cpp"], vec!["src/main.cpp"]),
    ] {
        for file in files {
            fs::write(root.join(file), source).unwrap();
        }

        let output = Command::new(env!("CARGO_BIN_EXE_breathers"))
            .current_dir(&root)
            .args(&arguments)
            .output()
            .unwrap();

        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );

        for file in files {
            assert_eq!(
                fs::read_to_string(root.join(file)).unwrap(),
                if changed.contains(&file) {
                    &expected
                } else {
                    source
                }
            );
        }

        let output = Command::new(env!("CARGO_BIN_EXE_breathers"))
            .current_dir(&root)
            .args(&arguments)
            .output()
            .unwrap();

        assert!(output.status.success());

        for file in changed {
            assert_eq!(fs::read_to_string(root.join(file)).unwrap(), expected);
        }
    }

    fs::remove_dir_all(root).unwrap();
}

#[test]
fn formats_valid_files_and_reports_syntax_errors() {
    let root = directory();
    let source = "int example(void) {\n    work();\n    return 1;\n}\n";
    let expected = source.replace("    return", "\n    return");

    for (name, invalid, formatted) in [
        ("unknown", source, source),
        ("broken.cpp", "int broken( {", expected.as_str()),
    ] {
        fs::write(root.join("main.c"), source).unwrap();
        fs::write(root.join(name), invalid).unwrap();

        let output = Command::new(env!("CARGO_BIN_EXE_breathers"))
            .current_dir(&root)
            .args(["main.c", name])
            .output()
            .unwrap();

        assert!(!output.status.success());
        assert!(String::from_utf8_lossy(&output.stderr).contains(name));
        assert_eq!(fs::read_to_string(root.join("main.c")).unwrap(), formatted);
        assert_eq!(fs::read_to_string(root.join(name)).unwrap(), invalid);
    }

    fs::write(root.join("main.c"), source).unwrap();
    fs::write(root.join("broken.rs"), "fn broken( {").unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_breathers"))
        .current_dir(&root)
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

    let source = "template<typename Value>\nValue example(Value value) {\n    work();\n    return value;\n}\n";
    fs::write(root.join("cplusplus.h"), source).unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_breathers"))
        .current_dir(&root)
        .arg("cplusplus.h")
        .output()
        .unwrap();

    assert!(!output.status.success());

    let output = Command::new(env!("CARGO_BIN_EXE_breathers"))
        .current_dir(&root)
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
        source.replace("    return", "\n    return")
    );

    fs::remove_dir_all(root).unwrap();
}
