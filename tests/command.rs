use std::{fs, process::Command};

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
        "target/ignored.c",
    ];

    for (arguments, changed) in [
        (
            vec![],
            vec!["main.c", "src/main.cpp", "src/header.h", "src/header.hpp"],
        ),
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
