use std::fs;

use crate::support::Workspace;

#[test]
fn filters_discovered_and_explicit_files_from_the_project_root() {
    let workspace = Workspace::new();
    let root = workspace.path();

    let sources = root.join("src");
    fs::create_dir_all(sources.join("generated")).unwrap();

    fs::write(
        root.join("breathers.toml"),
        r#"include = ["src/**/*.luau"]
exclude = ["**/generated/**", "src/omit.luau"]
[luau]
related = true
"#,
    )
    .unwrap();

    for file in ["generated/invalid.luau", "omit.luau", "other.lua"] {
        fs::write(sources.join(file), "local broken = {").unwrap();
    }

    let related = r"local value = 1
return value
";
    let separate = r"work()
return 1
";

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

        let output = workspace
            .command()
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
                r"work()

return 1
"
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
}

#[test]
fn formats_selected_files() {
    let workspace = Workspace::new();
    let root = workspace.path();
    fs::create_dir(root.join("src")).unwrap();
    fs::create_dir(root.join("target")).unwrap();
    let source = include_str!("../fixtures/c/command/standard/input.c");
    let expected = include_str!("../fixtures/c/command/standard/output.c");

    let files = [
        "main.c",
        "src/main.cpp",
        "src/header.h",
        "src/header.hpp",
        "target/generated.c",
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
        (vec!["target"], vec![]),
        (vec!["target/generated.c"], vec!["target/generated.c"]),
    ] {
        for file in files {
            fs::write(root.join(file), source).unwrap();
        }

        let output = workspace.command().args(&arguments).output().unwrap();

        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );

        for file in files {
            assert_eq!(
                fs::read_to_string(root.join(file)).unwrap(),
                if changed.contains(&file) {
                    expected
                } else {
                    source
                }
            );
        }

        let output = workspace.command().args(&arguments).output().unwrap();

        assert!(output.status.success());

        for file in changed {
            assert_eq!(fs::read_to_string(root.join(file)).unwrap(), expected);
        }
    }
}
