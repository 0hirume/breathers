use std::{collections::BTreeSet, ops::Range, path::Path, sync::Mutex};

use clang::{Clang, Entity, EntityKind, Index, Unsaved, diagnostic::Severity, token::TokenKind};

use crate::configuration::{Rule, Rules};

static PARSER: Mutex<Option<String>> = Mutex::new(None);

struct Token {
    range: Range<usize>,
    kind: TokenKind,
}

struct Formatter<'source, 'translation> {
    file: clang::source::File<'translation>,
    source: &'source str,
    rules: &'source Rules,
    tokens: Vec<Token>,
    macros: Vec<Range<usize>>,
    insertions: BTreeSet<usize>,
}

fn range(entity: Entity<'_>) -> Option<Range<usize>> {
    let range = entity.get_range()?;

    let start = range.get_start().get_expansion_location();
    let end = range.get_end().get_expansion_location();

    (start.file.is_some() && start.file == end.file && start.offset < end.offset)
        .then_some(start.offset as usize..end.offset as usize)
}

fn block(entity: Entity<'_>) -> Option<Rule> {
    match entity.get_kind() {
        EntityKind::IfStmt => Some(Rule::Conditionals),
        EntityKind::SwitchStmt => Some(Rule::Switches),
        EntityKind::ForStmt | EntityKind::ForRangeStmt => Some(Rule::ForLoops),
        EntityKind::WhileStmt => Some(Rule::WhileLoops),
        EntityKind::DoStmt | EntityKind::CompoundStmt => Some(Rule::DoBlocks),
        EntityKind::TryStmt => Some(Rule::TryBlocks),

        EntityKind::FunctionDecl
        | EntityKind::FunctionTemplate
        | EntityKind::Method
        | EntityKind::Constructor
        | EntityKind::Destructor
            if entity.is_definition() =>
        {
            Some(Rule::Functions)
        }

        _ => None,
    }
}

fn expression(entity: Entity<'_>) -> Option<Rule> {
    match entity.get_kind() {
        EntityKind::CallExpr => Some(Rule::Calls),
        EntityKind::InitListExpr => Some(Rule::Arrays),
        EntityKind::StringLiteral | EntityKind::CharacterLiteral => None,
        _ => entity.get_children().into_iter().find_map(expression),
    }
}

fn target_reads(entity: Entity<'_>, bindings: &[Entity<'_>], source: &str) -> bool {
    match entity.get_kind() {
        EntityKind::DeclRefExpr => false,

        EntityKind::ParenExpr | EntityKind::UnexposedExpr => entity
            .get_children()
            .into_iter()
            .any(|child| target_reads(child, bindings, source)),

        _ => reads(entity, bindings, source),
    }
}

fn reads(entity: Entity<'_>, bindings: &[Entity<'_>], source: &str) -> bool {
    match entity.get_kind() {
        EntityKind::CompoundStmt
        | EntityKind::LambdaExpr
        | EntityKind::FunctionDecl
        | EntityKind::FunctionTemplate
        | EntityKind::Method
        | EntityKind::ClassDecl
        | EntityKind::StructDecl => false,

        EntityKind::DeclRefExpr => entity
            .get_reference()
            .is_some_and(|reference| bindings.contains(&reference)),

        EntityKind::BinaryOperator => {
            let children = entity.get_children();

            if let [left, right, ..] = children.as_slice()
                && let (Some(left_range), Some(right_range)) = (range(*left), range(*right))
                && left_range.end <= right_range.start
                && source[left_range.end..right_range.start].trim() == "="
            {
                reads(*right, bindings, source) || target_reads(*left, bindings, source)
            } else {
                children
                    .into_iter()
                    .any(|child| reads(child, bindings, source))
            }
        }

        _ => entity
            .get_children()
            .into_iter()
            .any(|child| reads(child, bindings, source)),
    }
}

impl Formatter<'_, '_> {
    fn range(&self, entity: Entity<'_>) -> Option<Range<usize>> {
        let location = entity.get_range()?.get_start();

        if location.get_expansion_location().file != Some(self.file) {
            return None;
        }

        let offset = location.get_expansion_location().offset as usize;

        if let Some(expansion) = self.macros.iter().find(|range| range.contains(&offset)) {
            return Some(expansion.clone());
        }

        let mut range = range(entity)?;

        if let Some(expansion) = self
            .macros
            .iter()
            .find(|expansion| expansion.contains(&range.end))
        {
            range.end = expansion.end;
        }

        Some(range)
    }

    fn tokens(&self, range: &Range<usize>) -> &[Token] {
        let start = self
            .tokens
            .partition_point(|token| token.range.start < range.start);

        let end = self
            .tokens
            .partition_point(|token| token.range.end <= range.end);

        &self.tokens[start..end.max(start)]
    }

    fn multiline(&self, range: &Range<usize>) -> bool {
        let mut previous = range.start;

        for token in self.tokens(range) {
            if self.source[previous..token.range.start].contains('\n') {
                return true;
            }

            previous = token.range.end;
        }

        self.source[previous..range.end].contains('\n')
    }

    fn opaque(&self, entity: Entity<'_>) -> bool {
        matches!(
            entity.get_kind(),
            EntityKind::StringLiteral | EntityKind::CharacterLiteral
        ) || self.range(entity).is_some_and(|range| {
            self.macros
                .iter()
                .any(|expansion| expansion.contains(&range.start))
        })
    }

    fn complex(&self, entity: Entity<'_>) -> bool {
        if self.opaque(entity) {
            return false;
        }

        if matches!(
            entity.get_kind(),
            EntityKind::CaseStmt | EntityKind::DefaultStmt
        ) {
            return entity
                .get_children()
                .last()
                .is_some_and(|child| self.complex(*child));
        }

        if let Some(rule) = block(entity) {
            return self.rules.enabled(rule);
        }

        let rule = expression(entity).unwrap_or(
            if entity.is_declaration() || entity.get_kind() == EntityKind::DeclStmt {
                Rule::Declarations
            } else {
                Rule::Multiline
            },
        );

        self.rules.enabled(rule)
            && self
                .range(entity)
                .is_some_and(|range| self.multiline(&range))
    }

    fn written<'translation>(
        &self,
        entity: Entity<'translation>,
        bindings: &mut Vec<Entity<'translation>>,
    ) {
        match entity.get_kind() {
            EntityKind::VarDecl => bindings.push(entity),

            EntityKind::DeclRefExpr => {
                if let Some(reference) = entity.get_reference() {
                    bindings.push(reference);
                }
            }

            EntityKind::DeclStmt | EntityKind::MemberRefExpr => {
                for child in entity.get_children() {
                    self.written(child, bindings);
                }
            }

            EntityKind::BinaryOperator | EntityKind::CompoundAssignOperator => {
                let children = entity.get_children();

                if let [left, right, ..] = children.as_slice()
                    && let (Some(left_range), Some(right_range)) = (range(*left), range(*right))
                    && left_range.end <= right_range.start
                    && matches!(
                        self.source[left_range.end..right_range.start].trim(),
                        "=" | "+=" | "-=" | "*=" | "/=" | "%=" | "&=" | "|=" | "^=" | "<<=" | ">>="
                    )
                {
                    self.written(*left, bindings);
                }
            }

            EntityKind::UnaryOperator => {
                if let Some(range) = range(entity)
                    && self
                        .tokens(&range)
                        .iter()
                        .any(|token| matches!(&self.source[token.range.clone()], "++" | "--"))
                    && let Some(child) = entity.get_children().first()
                {
                    self.written(*child, bindings);
                }
            }

            _ => {}
        }
    }

    fn related(&self, previous: Entity<'_>, next: Entity<'_>) -> bool {
        let mut bindings = Vec::new();
        self.written(previous, &mut bindings);

        !bindings.is_empty() && reads(next, &bindings, self.source)
    }

    fn boundary(&self, start: usize, end: usize) -> Option<usize> {
        if start > end || end > self.source.len() {
            return None;
        }

        let mut start = start;
        let mut comments = Vec::new();

        for token in self.tokens(&(start..end)) {
            if token.kind == TokenKind::Comment {
                comments.push(token.range.clone());
            } else if comments.is_empty()
                && matches!(&self.source[token.range.clone()], ";" | "," | ")" | "]")
            {
                start = token.range.end;
            } else {
                return None;
            }
        }

        crate::syntax::boundary_ranges(self.source, start, end, comments.into_iter())
            .filter(|offset| !self.macros.iter().any(|range| range.contains(offset)))
    }

    fn space_first_statement(&mut self, entity: Entity<'_>, child: Entity<'_>) {
        if entity.get_kind() != EntityKind::CompoundStmt
            || entity
                .get_semantic_parent()
                .is_some_and(|parent| parent.get_kind() == EntityKind::Constructor)
            || !self.complex(child)
        {
            return;
        }

        let Some(current) = self.range(child) else {
            return;
        };

        let Some(body) = self.range(entity) else {
            return;
        };

        let Some(opening) = self
            .tokens(&body)
            .iter()
            .find(|token| &self.source[token.range.clone()] == "{")
        else {
            return;
        };

        if let Some(offset) = self.boundary(opening.range.end, current.start) {
            self.insertions.insert(offset);
        }
    }

    fn visit(&mut self, entity: Entity<'_>, root: bool) {
        if self.opaque(entity) || entity.get_kind() == EntityKind::CallExpr {
            return;
        }

        let children: Vec<_> = entity
            .get_children()
            .into_iter()
            .filter(|child| self.range(*child).is_some())
            .collect();

        if let Some(child) = children.first() {
            self.space_first_statement(entity, *child);
        }

        if root
            || matches!(
                entity.get_kind(),
                EntityKind::Namespace
                    | EntityKind::LinkageSpec
                    | EntityKind::CompoundStmt
                    | EntityKind::ClassDecl
                    | EntityKind::StructDecl
                    | EntityKind::UnionDecl
                    | EntityKind::ClassTemplate
                    | EntityKind::EnumDecl
            )
        {
            let mut previous = None;
            let mut case = None;

            for child in &children {
                let Some(current) = self
                    .range(*child)
                    .filter(|range| range.end <= self.source.len())
                else {
                    continue;
                };

                if matches!(
                    child.get_kind(),
                    EntityKind::MacroDefinition
                        | EntityKind::MacroExpansion
                        | EntityKind::InclusionDirective
                ) {
                    continue;
                }

                if let Some((left, preceding)) = previous.take() {
                    let left: Entity<'_> = left;
                    let preceding: Range<usize> = preceding;

                    let separate = match entity.get_kind() {
                        EntityKind::EnumDecl => {
                            self.rules.enabled(Rule::EnumMembers)
                                && (self.multiline(&preceding) || self.multiline(&current))
                        }

                        EntityKind::ClassDecl | EntityKind::ClassTemplate => {
                            self.rules.enabled(Rule::ClassMembers)
                                && (self.multiline(&preceding) || self.multiline(&current))
                        }

                        EntityKind::StructDecl | EntityKind::UnionDecl => {
                            self.rules.enabled(Rule::StructFields)
                                && (self.multiline(&preceding) || self.multiline(&current))
                        }

                        _ if matches!(
                            child.get_kind(),
                            EntityKind::CaseStmt | EntityKind::DefaultStmt
                        ) =>
                        {
                            case.is_some_and(|start| {
                                self.rules.enabled(Rule::SwitchCases)
                                    && self.multiline(&(start..preceding.end))
                            })
                        }

                        _ => {
                            self.complex(left)
                                || self.complex(*child)
                                || (child.get_kind() == EntityKind::ReturnStmt
                                    && self.rules.enabled(Rule::ReturnStatements))
                                || (self.source[current.clone()].starts_with("co_return")
                                    && self.rules.enabled(Rule::CoroutineReturns))
                        }
                    };

                    if left.get_kind() != EntityKind::AccessSpecifier
                        && child.get_kind() != EntityKind::AccessSpecifier
                        && separate
                        && !(self.rules.enabled(Rule::Related) && self.related(left, *child))
                        && let Some(offset) = self.boundary(preceding.end, current.start)
                    {
                        self.insertions.insert(offset);
                    }
                }

                if matches!(
                    child.get_kind(),
                    EntityKind::CaseStmt | EntityKind::DefaultStmt
                ) {
                    case = Some(current.start);
                }

                previous = Some((*child, current));
            }
        }

        for child in children {
            self.visit(child, false);
        }
    }
}

pub fn breathe(source: &str, path: Option<&Path>, rules: &Rules) -> Result<String, String> {
    let directory = std::env::current_dir().map_err(|error| error.to_string())?;
    let path = path.map_or_else(|| directory.join("source.cpp"), |path| directory.join(path));
    let name = path.to_str().ok_or("Clang requires a UTF-8 source path")?;

    let path = std::path::PathBuf::from(if let Some(network) = name.strip_prefix(r"\\?\UNC\") {
        format!(r"\\{network}")
    } else {
        name.strip_prefix(r"\\?\").unwrap_or(name).to_owned()
    });

    let mut arguments = crate::compilation::arguments(&path)?;

    if cfg!(windows) {
        arguments.push("-fno-delayed-template-parsing".into());
    }

    arguments.extend([
        "-x".into(),
        "c++".into(),
        "-working-directory".into(),
        path.parent()
            .ok_or("Source path has no directory")?
            .to_string_lossy()
            .into_owned(),
    ]);

    let mut guard = PARSER.lock().map_err(|error| error.to_string())?;

    if let Some(error) = guard.as_ref() {
        return Err(error.clone());
    }

    let clang = Clang::new().inspect_err(|error| {
        *guard = Some(error.clone());
    })?;

    let index = Index::new(&clang, false, false);

    let unit = index
        .parser(&path)
        .arguments(&arguments)
        .unsaved(&[Unsaved::new(&path, source)])
        .detailed_preprocessing_record(true)
        .parse()
        .map_err(|error| error.to_string())?;

    let errors: Vec<_> = unit
        .get_diagnostics()
        .into_iter()
        .filter(|diagnostic| diagnostic.get_severity() >= Severity::Error)
        .map(|diagnostic| diagnostic.formatter().format())
        .collect();

    if !errors.is_empty() {
        return super::c::breathe(source, &tree_sitter_cpp::LANGUAGE.into(), rules)
            .map_err(|_| format!("{}\nNo changes written", errors.join("\n")));
    }

    let file = unit
        .get_file(&path)
        .ok_or("Clang did not retain the source file")?;

    let length = u32::try_from(source.len()).map_err(|error| error.to_string())?;

    let tokens = clang::source::SourceRange::new(
        file.get_offset_location(0),
        file.get_offset_location(length),
    )
    .tokenize()
    .into_iter()
    .map(|token| Token {
        range: token.get_range().get_start().get_spelling_location().offset as usize
            ..token.get_range().get_end().get_spelling_location().offset as usize,
        kind: token.get_kind(),
    })
    .collect();

    let root = unit.get_entity();

    let macros = root
        .get_children()
        .into_iter()
        .filter(|entity| {
            entity.get_kind() == EntityKind::MacroExpansion
                && entity.get_range().is_some_and(|range| {
                    range.get_start().get_expansion_location().file == Some(file)
                })
        })
        .filter_map(range)
        .collect();

    let mut formatter = Formatter {
        file,
        source,
        rules,
        tokens,
        macros,
        insertions: BTreeSet::new(),
    };

    formatter.visit(root, true);

    Ok(crate::spacing::apply(source, formatter.insertions))
}

#[cfg(test)]
mod tests {
    #[test]
    fn formats_braced_defaults_and_preserves_source() {
        let source = "struct Value { Value() = default; };\nint example(Value parent = {}) {\n    int first = 1; // trailing\n    // attached\n    if (first) {\n        ++first;\n    }\n    return first;\n}\n";

        let expected = source
            .replace("int example", "\nint example")
            .replace("// trailing\n", "// trailing\n\n")
            .replace("    return", "\n    return");

        let rules = crate::configuration::Rules::default();
        assert_eq!(super::breathe(source, None, &rules).unwrap(), expected);
        assert_eq!(super::breathe(&expected, None, &rules).unwrap(), expected);

        assert_eq!(
            super::breathe(&source.replace('\n', "\r\n"), None, &rules).unwrap(),
            expected.replace('\n', "\r\n")
        );

        assert!(super::breathe("int broken( {", None, &rules).is_err());
    }

    #[test]
    fn spaces_first_control_flow_statement() {
        let source = "int resolve(\n    const int *from, int *expression, const int &) {\n    if (!from) {\n        return 0;\n    }\n    return expression ? *from : 0;\n}\n";

        let expected = source
            .replace(") {\n    if", ") {\n\n    if")
            .replace("}\n    return expression", "}\n\n    return expression");

        let rules = crate::configuration::Rules::default();

        assert_eq!(super::breathe(source, None, &rules).unwrap(), expected);
        assert_eq!(super::breathe(&expected, None, &rules).unwrap(), expected);

        assert_eq!(
            super::breathe(&source.replace('\n', "\r\n"), None, &rules).unwrap(),
            expected.replace('\n', "\r\n")
        );
    }

    #[test]
    fn leaves_first_constructor_loop_adjacent() {
        let source = "struct Sources {\n    Sources(int count) {\n        for (int index = 0; index < count; ++index) {\n            (void)index;\n        }\n    }\n};\n";

        let rules = crate::configuration::Rules::default();

        assert_eq!(super::breathe(source, None, &rules).unwrap(), source);

        let windows = source.replace('\n', "\r\n");
        assert_eq!(super::breathe(&windows, None, &rules).unwrap(), windows);
    }

    #[test]
    fn respects_related_reads_and_shadowed_bindings() {
        let configuration: crate::configuration::Configuration =
            toml::from_str("[\"c++\"]\nrelated = true\n").unwrap();

        let rules = &configuration.cplusplus;
        let source = "int example() {\n    int value = 1;\n    return value;\n}\n";
        assert_eq!(super::breathe(source, None, rules).unwrap(), source);

        for (source, next) in [
            (
                "void example() {\n    int value = 1;\n    value = (\n        2\n    );\n}\n",
                "    value =",
            ),
            (
                "void example() {\n    int index = 1;\n    for (int index = 0; index < 10; ++index) {\n    }\n}\n",
                "    for",
            ),
        ] {
            let formatted = super::breathe(source, None, rules).unwrap();
            assert!(formatted.contains(&format!("\n\n{next}")), "{formatted}");
            assert_eq!(super::breathe(&formatted, None, rules).unwrap(), formatted);
        }
    }

    #[test]
    fn preserves_spacing_rules_for_resolved_code() {
        for source in [
            "struct Value {\n    int first;\n    int (*callback)(\n        int,\n        int\n    );\n    int last;\n};\n",
            "int example(int value) {\n    switch (value) {\n        case 1:\n            ++value;\n            break;\n        default:\n            --value;\n            break;\n    }\n    return value;\n}\n",
            "int example() {\n    int values[] = {1, 2};\n    for (int value : values) {\n        if (value) {\n            return value;\n        }\n    }\n    return 0;\n}\n",
        ] {
            let rules = crate::configuration::Rules::default();

            let expected =
                crate::languages::c::breathe(source, &tree_sitter_cpp::LANGUAGE.into(), &rules)
                    .unwrap()
                    .replace("{\n    switch", "{\n\n    switch")
                    .replace("{\n        if", "{\n\n        if");

            assert_eq!(super::breathe(source, None, &rules).unwrap(), expected);
        }
    }

    #[test]
    fn preserves_macro_arguments() {
        let source = "#define IDENTITY(value) value\nint example() {\n    auto callback = IDENTITY([] {\n        int first = 1;\n        return first;\n    });\n    return callback();\n}\n";

        assert_eq!(
            super::breathe(source, None, &crate::configuration::Rules::default()).unwrap(),
            source
                .replace("() {\n    auto", "() {\n\n    auto")
                .replace("    return callback", "\n    return callback")
        );
    }

    #[test]
    fn preserves_macros_and_literal_contents() {
        let source = "#define BODY do { int first = 1; if (first) { ++first; } } while (0)\nint example() {\n    BODY;\n    const char *text = R\"(one\ntwo)\";\n    return text[0];\n}\n";
        let expected = source.replace("    return", "\n    return");

        assert_eq!(
            super::breathe(source, None, &crate::configuration::Rules::default()).unwrap(),
            expected
        );
    }
}
