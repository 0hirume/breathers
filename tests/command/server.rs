use std::fs;

use serde_json::json;
use tower_lsp_server::ls_types::Uri;

use crate::support::{Workspace, change, formatting, notification, open, protocol, request};

#[test]
fn serves_powershell_formatting_and_rejects_invalid_buffers() {
    let workspace = Workspace::new();
    let source = r#"function example {
    Write-Output "ready"
    return 1
}
"#;
    let buffer = source.replace("ready", "from-buffer");
    let expected = r#"function example {
    Write-Output "from-buffer"

    return 1
}
"#;

    let responses = protocol(
        &workspace,
        &["--lsp"],
        &[
            request(1, "initialize", json!({"capabilities": {}})),
            notification("initialized", json!({})),
            open("untitled:example", "powershell", &buffer),
            formatting(2, "untitled:example"),
            change("untitled:example", 2, "function broken {"),
            formatting(3, "untitled:example"),
            json!({"jsonrpc":"2.0","id":4,"method":"shutdown"}),
            json!({"jsonrpc":"2.0","method":"exit"}),
        ],
    );

    assert_eq!(responses[&2]["result"][0]["newText"], expected);
    assert!(responses[&3].get("error").is_some());
}

#[test]
fn serves_nushell_formatting() {
    let workspace = Workspace::new();
    let source = r"print ready
return done
";

    for language in ["nu", "nushell"] {
        let responses = protocol(
            &workspace,
            &["--lsp"],
            &[
                request(1, "initialize", json!({"capabilities": {}})),
                notification("initialized", json!({})),
                open("untitled:example", language, source),
                formatting(2, "untitled:example"),
                change("untitled:example", 2, "def broken [] {"),
                formatting(3, "untitled:example"),
                json!({"jsonrpc":"2.0","id":4,"method":"shutdown"}),
                json!({"jsonrpc":"2.0","method":"exit"}),
            ],
        );

        assert_eq!(
            responses[&2]["result"][0]["newText"],
            r"print ready

return done
"
        );

        assert!(responses[&3].get("error").is_some());
    }
}

#[test]
fn serves_document_formatting_with_buffer_synchronization() {
    let workspace = Workspace::new();
    let root = workspace.path();

    let project = root.join("project");
    fs::create_dir_all(project.join("src/generated")).unwrap();

    fs::write(
        root.join("breathers.toml"),
        r"[luau]
returns = false
",
    )
    .unwrap();

    fs::write(
        project.join("breathers.toml"),
        r#"include = ["src/**"]
exclude = ["**/generated/**"]
"#,
    )
    .unwrap();

    let disk = r"local disk = 1
return disk
";
    fs::write(project.join("src/main.luau"), disk).unwrap();
    let uri = Uri::from_file_path(project.join("src/main.luau")).unwrap();
    let excluded = Uri::from_file_path(project.join("src/generated/example.luau")).unwrap();
    let source = "local value = 1\r\nreturn '🙂'";
    let formatted = "local value = 1\r\n\r\nreturn '🙂'";

    let spaced = r#"local first = require("shared/first")

local second = require("shared/second")
"#;

    let responses = protocol(
        &workspace,
        &["--lsp"],
        &[
            request(1, "initialize", json!({"capabilities": {}})),
            notification("initialized", json!({})),
            open(&uri, "luau", source),
            formatting(2, &uri),
            change(&uri, 2, formatted),
            change(&uri, 1, "local broken = {"),
            formatting(3, &uri),
            change(&uri, 3, "local broken = {"),
            formatting(4, &uri),
            notification(
                "textDocument/didChange",
                json!({
                    "textDocument": {
                        "uri": uri,
                        "version": 4
                    },
                    "contentChanges": [
                        {
                            "range": {
                                "start": {
                                    "line": 0,
                                    "character": 0
                                },
                                "end": {
                                    "line": 0,
                                    "character": 1
                                }
                            },
                            "text": "x"
                        }
                    ]
                }),
            ),
            formatting(5, &uri),
            change(&uri, 5, spaced),
            formatting(6, &uri),
            notification(
                "textDocument/didClose",
                json!({
                    "textDocument": {
                        "uri": uri
                    }
                }),
            ),
            formatting(7, &uri),
            open(&excluded, "luau", source),
            formatting(8, &excluded),
            open("untitled:example", "luau", source),
            formatting(9, "untitled:example"),
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
}

#[test]
fn applies_explicit_server_configuration_and_rejects_mixed_modes() {
    let workspace = Workspace::new();
    let root = workspace.path();

    fs::create_dir_all(root.join("nested")).unwrap();

    fs::write(
        root.join("nested/breathers.toml"),
        r"[luau]
returns = false
",
    )
    .unwrap();

    let uri = Uri::from_file_path(root.join("nested/example.luau")).unwrap();

    let source = r"local value = 1
return value
";

    for (configuration, expected) in [
        (
            r"[luau]
related = true
",
            None,
        ),
        (
            r"[luau]
related = false
",
            Some(
                r"local value = 1

return value
",
            ),
        ),
        ("include = []\n", None),
        (
            r"[luau]
unknown = true
",
            None,
        ),
    ] {
        fs::write(root.join("selected.toml"), configuration).unwrap();

        let responses = protocol(
            &workspace,
            &["--lsp", "-c", "selected.toml"],
            &[
                request(1, "initialize", json!({"capabilities": {}})),
                notification("initialized", json!({})),
                open(&uri, "unknown", source),
                formatting(2, &uri),
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
        let output = workspace.command().args(arguments).output().unwrap();

        assert!(!output.status.success());
        assert_eq!(output.stdout, Vec::<u8>::new());
    }

    assert!(!root.join("nested/example.luau").exists());
}
