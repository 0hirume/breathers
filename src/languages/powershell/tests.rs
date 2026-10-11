use crate::configuration::{Configuration, Rules};

#[test]
fn spaces_functions_named_blocks_and_returns() {
    let source = include_str!("../../../tests/fixtures/powershell/spacing/input.ps1");

    let expected = include_str!("../../../tests/fixtures/powershell/spacing/output.ps1");

    for (source, expected) in [
        (source.to_owned(), expected.to_owned()),
        (expected.to_owned(), expected.to_owned()),
        (source.replace('\n', "\r\n"), expected.replace('\n', "\r\n")),
    ] {
        assert_eq!(
            super::breathe(&source, &Rules::default()).unwrap(),
            expected
        );
    }
}

#[test]
fn preserves_literals_compact_code_and_pipeline_stages() {
    for source in [
        r"$text = @'
first
if ($true) { text }
'@
Write-Output done
",
        r#"$text = @"
first
$(if ($true) { Write-Output text })
"@
Write-Output done
"#,
        r#"$text = "first
$(Write-Output text; return 1)"
Write-Output done
"#,
        r"Get-Process `
    | Sort-Object CPU
",
        r"Get-Process
| Sort-Object CPU
",
        r"Write-Output first; if ($true) { Write-Output yes }; return 1
",
        r"$result = $null ?? ($true ? 1 : 2)
Write-Output $result
",
        r"Write-Output first


return 1",
    ] {
        assert_eq!(super::breathe(source, &Rules::default()).unwrap(), source);
    }
}

#[test]
fn rejects_invalid_syntax() {
    super::breathe("function Broken {", &Rules::default()).unwrap_err();
}

#[test]
fn related_preserves_interpolated_variable_dependencies() {
    let configuration: Configuration = toml::from_str(
        r"[powershell]
related = true
",
    )
    .unwrap();

    let source = include_str!("../../../tests/fixtures/powershell/interpolation/input.ps1");

    assert_eq!(
        super::breathe(source, &configuration.powershell).unwrap(),
        source
    );

    assert_eq!(
        super::breathe(source, &Rules::default()).unwrap(),
        include_str!("../../../tests/fixtures/powershell/interpolation/output.ps1")
    );
}
