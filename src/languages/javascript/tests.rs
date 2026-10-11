use crate::configuration::Rules;

#[test]
fn spaces_javascript_typescript_and_tsx() {
    let source = r"function example() {
    const first = 1;
    const second = 2; // trailing
    // attached
    if (first < second) {
        work();
    } else {
        other();
    }
    save();
    return first;
}
";

    let expected = r"function example() {
    const first = 1;
    const second = 2; // trailing

    // attached
    if (first < second) {
        work();
    } else {
        other();
    }

    save();

    return first;
}
";

    for language in [
        tree_sitter_javascript::LANGUAGE.into(),
        tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into(),
        tree_sitter_typescript::LANGUAGE_TSX.into(),
    ] {
        assert_eq!(
            super::breathe(source, &language, &Rules::default()).unwrap(),
            expected
        );

        assert_eq!(
            super::breathe(expected, &language, &Rules::default()).unwrap(),
            expected
        );

        assert_eq!(
            super::breathe(&source.replace('\n', "\r\n"), &language, &Rules::default()).unwrap(),
            expected.replace('\n', "\r\n")
        );
    }
}

#[test]
fn preserves_literals_and_selects_typescript_grammars() {
    for source in [
        r"const text = `first
second`;
finish();
",
        r"const element = <div>first
second</div>;
finish();
",
        r"const pattern = /foo[{}]/;
finish();
",
    ] {
        assert_eq!(
            super::breathe(
                source,
                &tree_sitter_javascript::LANGUAGE.into(),
                &Rules::default()
            )
            .unwrap(),
            source
        );
    }

    let source = r"const value = <number>result;
finish();
";

    assert_eq!(
        super::breathe(
            source,
            &tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into(),
            &Rules::default()
        )
        .unwrap(),
        source
    );

    super::breathe(
        source,
        &tree_sitter_typescript::LANGUAGE_TSX.into(),
        &Rules::default(),
    )
    .unwrap_err();

    let source = r"const element: Element = <div>text</div>;
finish();
";

    assert_eq!(
        super::breathe(
            source,
            &tree_sitter_typescript::LANGUAGE_TSX.into(),
            &Rules::default()
        )
        .unwrap(),
        source
    );

    super::breathe(
        "function broken( {",
        &tree_sitter_javascript::LANGUAGE.into(),
        &Rules::default(),
    )
    .unwrap_err();
}
