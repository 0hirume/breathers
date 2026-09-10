use std::{
    fs,
    io::Write,
    process::{Command, Output, Stdio},
};

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

#[test]
fn formats_standard_input_without_writing_files() {
    let root = std::env::temp_dir()
        .join("breathers")
        .join(std::process::id().to_string())
        .join(
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
                .to_string(),
        );

    fs::create_dir_all(&root).unwrap();
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
    let root = std::env::temp_dir()
        .join("breathers")
        .join(std::process::id().to_string())
        .join(
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
                .to_string(),
        );

    fs::create_dir_all(&root).unwrap();
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
    let root = std::env::temp_dir().join(format!(
        "breathers-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));

    fs::create_dir(&root).unwrap();

    let sources = [
        ("example.py", "def example():\n    work()\n    return 1\n"),
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
        "[python]\nreturns = false\n[typescript]\nreturns = false\n",
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
    let root = std::env::temp_dir()
        .join("breathers")
        .join(std::process::id().to_string())
        .join(
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
                .to_string(),
        );

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
    let root = std::env::temp_dir().join(format!(
        "breathers-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));

    fs::create_dir(&root).unwrap();
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
        source
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
        source
    );

    fs::remove_dir_all(root).unwrap();
}

#[test]
fn formats_selected_files_and_rejects_errors_before_writing() {
    let root = std::env::temp_dir().join(format!(
        "breathers-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));

    fs::create_dir(&root).unwrap();
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

    for (name, invalid) in [("unknown", source), ("broken.cpp", "int broken( {")] {
        fs::write(root.join("main.c"), source).unwrap();
        fs::write(root.join(name), invalid).unwrap();

        let output = Command::new(env!("CARGO_BIN_EXE_breathers"))
            .current_dir(&root)
            .args(["main.c", name])
            .output()
            .unwrap();

        assert!(!output.status.success());
        assert_eq!(fs::read_to_string(root.join("main.c")).unwrap(), source);
    }

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
