use std::collections::BTreeSet;

use tree_sitter::Node;

use crate::configuration::{Rule, Rules};

fn opaque(node: Node<'_>) -> bool {
    matches!(node.kind(), "string" | "comment" | "hash_bang_line")
}

fn block(node: Node<'_>) -> Option<Rule> {
    match node.kind() {
        "if_statement" => Some(Rule::Conditionals),
        "while_statement" => Some(Rule::WhileLoops),
        "repeat_statement" => Some(Rule::RepeatLoops),
        "for_statement" => Some(Rule::ForLoops),
        "do_statement" => Some(Rule::DoBlocks),
        "function_declaration" | "function_definition" => Some(Rule::Functions),
        _ => None,
    }
}

fn expression(node: Node<'_>) -> Option<Rule> {
    match node.kind() {
        "function_call" => Some(Rule::Calls),
        "table_constructor" => Some(Rule::Tables),
        _ => None,
    }
}

fn complex(node: Node<'_>, source: &str, rules: &Rules) -> bool {
    if let Some(rule) = block(node) {
        rules.enabled(rule)
    } else {
        let rule = crate::syntax::rule(node, expression, opaque).unwrap_or(
            if node.kind() == "variable_declaration" {
                Rule::Declarations
            } else {
                Rule::Multiline
            },
        );

        rules.enabled(rule) && crate::syntax::multiline(node, source, opaque)
    }
}

fn visit(node: Node<'_>, source: &str, rules: &Rules, insertions: &mut BTreeSet<usize>) {
    if opaque(node) {
        return;
    }

    let mut cursor = node.walk();
    let children: Vec<_> = node.named_children(&mut cursor).collect();

    if matches!(node.kind(), "chunk" | "block") {
        let mut previous = None;

        for (index, child) in children.iter().enumerate() {
            if matches!(child.kind(), "comment" | "empty_statement") {
                continue;
            }

            if let Some(previous) = previous {
                let left: Node<'_> = children[previous];

                if left.kind() != "hash_bang_line"
                    && (complex(left, source, rules)
                        || complex(*child, source, rules)
                        || (rules.enabled(Rule::ReturnStatements)
                            && child.kind() == "return_statement"))
                    && !(rules.enabled(Rule::Related)
                        && crate::syntax::related(left, *child, source))
                    && let Some(offset) = crate::syntax::boundary(
                        source,
                        left.end_byte(),
                        child.start_byte(),
                        &children[previous + 1..index],
                    )
                {
                    insertions.insert(offset);
                }
            }

            previous = Some(index);
        }
    }

    for child in children {
        visit(child, source, rules, insertions);
    }
}

pub fn breathe(source: &str, rules: &Rules) -> Result<String, String> {
    crate::syntax::breathe(source, &tree_sitter_lua::LANGUAGE.into(), rules, visit)
}

#[cfg(test)]
mod tests {
    fn breathe(source: &str) -> Result<String, String> {
        super::breathe(source, &crate::configuration::Rules::default())
    }

    #[test]
    fn spaces_blocks_calls_tables_and_returns() {
        let source = "local first = 1\nlocal second = 2 -- trailing\n-- attached\nif first < second then\n    work()\nelseif first == second then\n    same()\nelse\n    other()\nend\nlocal values = {\n    first,\n    second,\n}\nsave(\n    values\n)\nreturn values\n";

        let expected = source
            .replace("-- trailing\n", "-- trailing\n\n")
            .replace("end\n", "end\n\n")
            .replace("}\n", "}\n\n")
            .replace("return values", "\nreturn values");

        assert_eq!(breathe(source).unwrap(), expected);
        assert_eq!(breathe(&expected).unwrap(), expected);

        assert_eq!(
            breathe(&source.replace('\n', "\r\n")).unwrap(),
            expected.replace('\n', "\r\n")
        );

        assert!(breathe("local broken = {").is_err());
        assert!(breathe("local value: number = 1").is_err());
    }

    #[test]
    fn supports_lua_syntax_and_nested_functions() {
        let source = "local value <const> = 8 // 2\n::again::\nvalue = value & 3\nif value > 0 then\n    goto again\nend\nreturn value\n";

        let expected = source
            .replace("if value", "\nif value")
            .replace("return value", "\nreturn value");

        assert_eq!(breathe(source).unwrap(), expected);

        let source =
            "local function example()\n    work()\n    return 1\nend\nlocal value = example()\n";

        let expected = source
            .replace("    return", "\n    return")
            .replace("end\n", "end\n\n");

        assert_eq!(breathe(source).unwrap(), expected);
    }

    #[test]
    fn preserves_strings_comments_and_compact_code() {
        for source in [
            "local text = [=[first\nsecond]=]\nwork()\n",
            "local value = 1 --[[first\nsecond]]\nwork()\n",
            "local first = 1; if first > 0 then work() end; finish()\n",
            "#!/usr/bin/env lua\nlocal first = 1\nlocal second = 2\n",
            "local first = 1\n\nif first > 0 then\n    work()\nend\n",
        ] {
            assert_eq!(breathe(source).unwrap(), source);
        }
    }
}
