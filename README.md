# breathers

Give code some breathing room. **breathers** plays on _breathers_ and _breathe-rs_: breathe Rust.

Automatically add blank lines around control flow, blocks, and multiline statements. Keep consecutive simple statements together.

## Install

```sh
cargo install --git https://github.com/0hirume/breathers --locked
```

Building from source requires Rust and a C/C++ compiler for the syntax parsers.

## Usage

Run your usual formatter, then choose what to space:

```sh
breathers
breathers src/
breathers src/main.rs
breathers src/main.luau
breathers src/main.lua
breathers src/main.c src/main.cpp
breathers src/main.py src/main.js src/main.ts
```

With no paths, breathers searches the current directory recursively. Directory arguments search that directory recursively; file arguments process only those files. Files are edited in place, and overlapping paths are processed once. Searches skip `.git` and symbolic links.

The file extension selects the language:

| Language         | Extensions                                                               |
| ---------------- | ------------------------------------------------------------------------ |
| Rust             | `.rs`                                                                    |
| Lua              | `.lua`                                                                   |
| Luau             | `.luau`                                                                  |
| C                | `.c`, `.h`                                                               |
| C++              | `.cpp`, `.cc`, `.cxx`, `.c++`, `.hpp`, `.hh`, `.hxx`, `.h++`, `.C`, `.H` |
| Python           | `.py`, `.pyi`                                                            |
| JavaScript / JSX | `.js`, `.jsx`, `.mjs`, `.cjs`                                            |
| TypeScript       | `.ts`, `.mts`, `.cts`                                                    |
| TSX              | `.tsx`                                                                   |

Use `--language` or `-l` to override detection for every selected file:

```sh
breathers -l c++ include/header.h
breathers --language c source
breathers -l luau src/module.lua
```

Explicit files with unknown extensions require a language. Directory searches select the extensions listed above. `.lua` selects Lua; use `-l luau` for Luau projects using `.lua` filenames. There is no fallback language for unknown extensions.

Use `breathers --help` for help.

### Standard input

Use `-` as the only input path and specify `--language` / `-l`:

```sh
breathers -l luau -
```

Reads UTF-8 source from stdin and writes formatted source to stdout without editing files. Diagnostics go to stderr. Configuration loading and spacing rules apply as usual. Include/exclude globs are validated but do not filter stdin. Line endings and the presence or absence of a final newline are preserved.

## Configuration

Project configuration comes from the nearest `breathers.toml`, searching the current working directory and then its ancestors. Select a different project configuration with `-c` / `--config`:

```sh
breathers -c settings.toml src/
```

Global configuration is loaded first, when present:

| OS      | Global file                                                                    |
| ------- | ------------------------------------------------------------------------------ |
| Windows | Roaming AppData (`%APPDATA%`)/`breathers/config.toml`                          |
| macOS   | `~/Library/Application Support/breathers/config.toml`                          |
| Linux   | `$XDG_CONFIG_HOME/breathers/config.toml`, or `~/.config/breathers/config.toml` |

Project values override matching global keys. Language sections merge by individual toggle; supplied include/exclude lists replace the corresponding global lists. Granular-rule inheritance is resolved after merging. Only the nearest project file is loaded, not every ancestor. `-c` replaces project discovery and still inherits global settings. Configuration directories and files are not created automatically.

Add the schema declaration at the top for editor completion and validation:

```toml
#:schema https://raw.githubusercontent.com/0hirume/breathers/main/schema.json

[rust]
returns = false

[python]
blocks = false

[javascript]
fields = false

[typescript]
arms = false
```

Sections are `rust`, `lua`, `luau`, `c`, `"c++"`, `python`, `javascript`, and `typescript`. JSX uses `javascript` settings; TSX uses `typescript` settings.

The broad toggles remain available as groups. Groups are enabled when omitted; language-specific toggles inherit their group unless explicitly set:

| Toggle      | Controls                                                          | Languages                                    |
| ----------- | ----------------------------------------------------------------- | -------------------------------------------- |
| `blocks`    | Spacing around control flow and block declarations                | All                                          |
| `multiline` | Spacing around other multiline statements                         | All                                          |
| `returns`   | Spacing before noninitial returns; also Rust final expressions    | All                                          |
| `fields`    | Spacing between multiline fields, members, or dictionary entries  | Rust, C, C++, Python, JavaScript, TypeScript |
| `arms`      | Spacing between multiline match arms or switch cases              | Rust, C, C++, Python, JavaScript, TypeScript |
| `variants`  | Spacing between multiline enum entries                            | Rust, C, C++, TypeScript                     |
| `groups`    | Service, require, type, class, and populated-declaration grouping | Luau                                         |

### File selection

Top-level globs filter both recursively discovered files and explicit file arguments. Exclusions win over inclusions:

```toml
include = ["src/**", "tests/**"]
exclude = ["**/generated/**", "**/vendor/**"]
```

Patterns match paths relative to the selected project configuration's directory, or the current working directory when there is no project configuration. Global patterns use that same root. Paths outside the root are matched as absolute paths.

Omitting `include` leaves all otherwise eligible files selected; `include = []` selects none. Omitting `exclude` excludes nothing beyond the existing directory-search skips; `exclude = []` clears inherited exclusions. Globs do not add support for unknown file extensions or change language detection.

Matching uses globset's case-sensitive defaults: `*` can cross directory separators, `**/` matches any number of directories, and patterns support `?`, character classes, and alternatives such as `*.{lua,luau}`. Use `/` in patterns on every OS. Matching excluded directories are skipped during traversal. Invalid globs stop processing before source files are written.

### Related statements

Enable `related` in any language section to keep adjacent variable dependencies together:

```toml
[luau]
related = true
```

```luau
local values = {}
for key, value in values do
    process(key, value)
end
```

This is not limited to loops. It also covers declarations or assignments followed by conditions, calls, returns, further assignments, and other expressions reading those variables. Multiple bindings and field/index receivers are recognized where supported. Each adjacent pair is considered independently, so dependency chains can stay together.

The rule suppresses newly inserted blank lines; it preserves existing blank lines and comments. It is disabled by default and independent of the broad groups. Matching uses syntax-tree identifiers, not substrings, property names, or literal text. Control-flow headers are inspected without treating references inside their bodies as immediate dependencies. This is conservative syntax analysis, not macro expansion or full symbol, type, or alias resolution.

### Granular overrides

Keep a group enabled and disable individual constructs, or disable a group and re-enable only the constructs you want:

```toml
[rust]
returns = false
tail_expressions = true
enum_variants = false

[python]
blocks = false
decorated_functions = true
match_cases = false

[typescript]
fields = false
interface_members = true
type_aliases = false
```

An explicit granular value takes precedence over its group in either direction. Omitted granular values inherit their group, and each language exposes only its relevant granular controls. The schema provides the exact choices for each section; a granular key from another language is rejected.

| Group       | Granular controls, where supported                                                                                                                                                                                                |
| ----------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `blocks`    | `conditionals`, `for_loops`, `while_loops`, `repeat_loops`, `loop_expressions`, `do_blocks`, `block_expressions`, `matches`, `switches`, `try_blocks`, `with_blocks`, `functions`, `decorated_functions`, `classes`, `interfaces` |
| `multiline` | `calls`, `arrays`, `tables`, `objects`, `declarations`, `type_aliases`                                                                                                                                                            |
| `returns`   | `return_statements`, `tail_expressions`, `coroutine_returns`                                                                                                                                                                      |
| `fields`    | `struct_fields`, `tuple_fields`, `class_members`, `interface_members`, `type_members`, `object_properties`, `dictionary_entries`                                                                                                  |
| `arms`      | `match_arms`, `match_cases`, `switch_cases`                                                                                                                                                                                       |
| `variants`  | `enum_variants`, `enum_members`                                                                                                                                                                                                   |
| `groups`    | `services`, `requires`, `type_groups`, `class_groups`, `populated_declarations`                                                                                                                                                   |

Rust has separate controls for return statements and tail expressions. Python distinguishes decorated functions from ordinary function declarations. TypeScript distinguishes interface members, type members, and class members. Luau has individual controls for module grouping.

For multiline statements, type aliases and recognized call or collection expressions use their specific controls; `declarations` covers other multiline declarations. Recognized blocks use their `blocks` controls rather than falling back to `multiline`. An enabled rule on a neighboring statement can still introduce spacing. Disabling rules preserves existing blank lines.

An explicitly selected configuration must exist. Invalid global or project configuration stops processing before any source files are written.

The schema is stored in [`schema.json`](schema.json). `breathers --schema` prints the schema without formatting files.

## Example

Before:

```rust
fn example() {
    let ready = true;
    let count = 2;
    if ready {
        process(count);
    }
    finish();
}
```

After:

```rust
fn example() {
    let ready = true;
    let count = 2;

    if ready {
        process(count);
    }

    finish();
}
```

## Behavior

- Adds space around control flow, blocks, and multiline statements.
- Adds a blank line before a return when another statement precedes it.
- Separates multiline fields, members, and enum entries where supported, keeping consecutive simple entries together.
- Keeps existing blank lines and leaves block edges alone.
- Preserves comment text, strings, and line endings.
- Leaves compact single-line code alone. Repeated runs leave the result unchanged.

Rust also gets spacing between multiline match arms and before noninitial final expressions.

Lua uses a dedicated syntax parser to separate blocks, functions, multiline calls and tables, and noninitial returns. Lua and Luau are parsed separately.

Luau uses vermis to separate blocks, functions, multiline calls, tables, and types, and to add spacing before noninitial returns. At module scope, it separates service, require, type, and class groups. Consecutive requires sharing a path prefix stay together, as do simple declarations and their indexed assignments. Existing blank lines are preserved.

C and C++ get spacing between multiline switch cases. Preprocessor directives and macro definitions are preserved. Function-call arguments are left untouched because macro invocations can look like ordinary calls. Nested blocks inside conditional compilation can be formatted, but breathers does not expand macros or preprocess branches. Code that the syntax parser rejects is reported as an error.

Python preserves indentation, decorators, and string contents. JavaScript and TypeScript preserve strings, regular expressions, and JSX content. TypeScript and TSX use separate grammars; use `-l tsx` to explicitly select TSX.

Unreadable files, invalid configuration, unknown languages, and syntax errors stop the command before any files are written. A write failure can leave earlier files updated.
