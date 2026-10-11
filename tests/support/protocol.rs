use std::{
    collections::BTreeMap,
    io::{BufRead as _, BufReader, Read as _, Write as _},
    process::Stdio,
};

use serde::Serialize;
use serde_json::{Value, json};

use super::Workspace;

pub(crate) fn protocol(
    workspace: &Workspace,
    arguments: &[&str],
    messages: &[Value],
) -> BTreeMap<i64, Value> {
    let mut child = workspace
        .command()
        .args(arguments)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();

    let mut sender = child.stdin.take().unwrap();
    let mut receiver = BufReader::new(child.stdout.take().unwrap());
    let mut responses = BTreeMap::new();

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
            let response: Value = serde_json::from_slice(&body).unwrap();
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

pub(crate) fn request(identifier: i64, method: &str, parameters: Value) -> Value {
    let mut message = json!({
        "jsonrpc": "2.0",
        "id": identifier,
        "method": method,
    });

    message["params"] = parameters;

    message
}

pub(crate) fn notification(method: &str, parameters: Value) -> Value {
    let mut message = json!({
        "jsonrpc": "2.0",
        "method": method,
    });

    message["params"] = parameters;

    message
}

pub(crate) fn open(uri: impl Serialize, language: &str, source: &str) -> Value {
    notification(
        "textDocument/didOpen",
        json!({
            "textDocument": {
                "uri": uri,
                "languageId": language,
                "version": 1,
                "text": source,
            },
        }),
    )
}

pub(crate) fn formatting(identifier: i64, uri: impl Serialize) -> Value {
    request(
        identifier,
        "textDocument/formatting",
        json!({
            "textDocument": { "uri": uri },
            "options": { "tabSize": 4, "insertSpaces": true },
        }),
    )
}

pub(crate) fn change(uri: impl Serialize, version: i32, source: &str) -> Value {
    notification(
        "textDocument/didChange",
        json!({
            "textDocument": { "uri": uri, "version": version },
            "contentChanges": [{ "text": source }],
        }),
    )
}
