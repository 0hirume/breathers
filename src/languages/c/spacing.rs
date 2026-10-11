use std::{
    collections::BTreeSet,
    env,
    ops::Range,
    path::{Path, PathBuf},
    sync::Mutex,
};

use clang::{
    Clang, Entity, EntityKind, Index, Unsaved,
    diagnostic::Severity,
    source::{File, SourceRange},
    token::TokenKind,
};

use crate::{
    compilation,
    configuration::{Rule, Rules},
    languages::cplusplus,
    spacing,
};

static PARSER: Mutex<Option<String>> = Mutex::new(None);

#[derive(Copy, Clone, Eq, PartialEq)]
pub(crate) enum Dialect {
    C,
    CPlusPlus,
}

impl Dialect {
    fn argument(self) -> &'static str {
        match self {
            Self::C => "c",
            Self::CPlusPlus => "c++",
        }
    }

    fn filename(self) -> &'static str {
        match self {
            Self::C => "source.c",
            Self::CPlusPlus => "source.cpp",
        }
    }
}

struct Token {
    range: Range<usize>,
    kind: TokenKind,
}

struct Formatter<'source, 'translation> {
    file: File<'translation>,
    source: &'source str,
    rules: &'source Rules,
    tokens: Vec<Token>,
    macros: Vec<Range<usize>>,
    insertions: BTreeSet<usize>,
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

        spacing::boundary(self.source, start, end, comments.into_iter())
            .filter(|offset| !self.macros.iter().any(|range| range.contains(offset)))
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

pub(crate) fn breathe(
    source: &str,
    path: Option<&Path>,
    rules: &Rules,
    language: Dialect,
) -> Result<String, String> {
    let directory = env::current_dir().map_err(|error| error.to_string())?;

    let path = path.map_or_else(
        || directory.join(language.filename()),
        |path| directory.join(path),
    );

    let name = path.to_str().ok_or("Clang requires a UTF-8 source path")?;

    let path = PathBuf::from(if let Some(network) = name.strip_prefix(r"\\?\UNC\") {
        format!(r"\\{network}")
    } else {
        name.strip_prefix(r"\\?\").unwrap_or(name).to_owned()
    });

    let mut guard = PARSER.lock().map_err(|error| error.to_string())?;

    if let Some(error) = guard.as_ref() {
        return Err(error.clone());
    }

    let clang = Clang::new().inspect_err(|error| {
        *guard = Some(error.clone());
    })?;

    let mut arguments = compilation::arguments(&path)?;

    if language == Dialect::CPlusPlus && cfg!(windows) {
        arguments.push("-fno-delayed-template-parsing".into());
    }

    arguments.extend(["-x".into(), language.argument().into()]);

    let index = Index::new(&clang, false, false);

    let unit = index
        .parser(&path)
        .arguments(&arguments)
        .unsaved(&[Unsaved::new(&path, source)])
        .detailed_preprocessing_record(true)
        .parse()
        .map_err(|error| error.to_string())?;

    let file = unit
        .get_file(&path)
        .ok_or("Clang did not retain the source file")?;

    let root = unit.get_entity();

    let macros: Vec<_> = root
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

    let errors: Vec<_> = unit
        .get_diagnostics()
        .into_iter()
        .filter(|diagnostic| diagnostic.get_severity() >= Severity::Error)
        .map(|diagnostic| diagnostic.formatter().format())
        .collect();

    if !errors.is_empty() {
        if language == Dialect::CPlusPlus {
            let protected: Vec<_> = file
                .get_skipped_ranges()
                .into_iter()
                .map(|range| {
                    let start = range.get_start().get_expansion_location().offset as usize;
                    let end = range.get_end().get_expansion_location().offset as usize;

                    start..end
                })
                .chain(macros.iter().cloned())
                .collect();

            return cplusplus::breathe(source, rules, &protected)
                .map_err(|error| format!("{}\n{error}\nNo changes written", errors.join("\n")));
        }

        return Err(format!("{}\nNo changes written", errors.join("\n")));
    }

    let length = u32::try_from(source.len()).map_err(|error| error.to_string())?;

    let tokens = SourceRange::new(
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

    let mut formatter = Formatter {
        file,
        source,
        rules,
        tokens,
        macros,
        insertions: BTreeSet::new(),
    };

    formatter.visit(root, true);

    Ok(spacing::apply(source, formatter.insertions))
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
