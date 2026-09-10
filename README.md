# breathers

Give Rust, Lua, Luau, C, and C++ code some breathing room. **breathers** plays on *breathers* and *breathe-rs*: breathe Rust.

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
```

With no paths, breathers searches the current directory recursively. Directory arguments search that directory recursively; file arguments process only those files. Files are edited in place, and overlapping paths are processed once. Searches skip `.git`, `target`, and symbolic links.

The file extension selects the language:

| Language | Extensions |
| --- | --- |
| Rust | `.rs` |
| Lua | `.lua` |
| Luau | `.luau` |
| C | `.c`, `.h` |
| C++ | `.cpp`, `.cc`, `.cxx`, `.c++`, `.hpp`, `.hh`, `.hxx`, `.h++`, `.C`, `.H` |

Use `--language` or `-l` to override detection for every selected file:

```sh
breathers -l c++ include/header.h
breathers --language c source
breathers -l luau src/module.lua
```

Explicit files with unknown extensions require a language. Directory searches select the extensions listed above. `.lua` selects Lua; use `-l luau` for Luau projects using `.lua` filenames. There is no fallback language for unknown extensions.

Use `breathers --help` for help.

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
- Separates multiline fields and enum entries in Rust, C, and C++, keeping consecutive simple entries together.
- Keeps existing blank lines and leaves block edges alone.
- Preserves comment text, strings, and line endings.
- Leaves compact single-line code alone. Repeated runs leave the result unchanged.

Rust also gets spacing between multiline match arms and before noninitial final expressions.

Lua uses a dedicated syntax parser to separate blocks, functions, multiline calls and tables, and noninitial returns. Lua and Luau are parsed separately.

Luau uses vermis to separate blocks, functions, multiline calls, tables, and types, and to add spacing before noninitial returns. At module scope, it separates service, require, type, and class groups. Consecutive requires sharing a path prefix stay together, as do simple declarations and their indexed assignments. Existing blank lines are preserved.

C and C++ get spacing between multiline switch cases. Preprocessor directives and macro definitions are preserved. Function-call arguments are left untouched because macro invocations can look like ordinary calls. Nested blocks inside conditional compilation can be formatted, but breathers does not expand macros or preprocess branches. Code that the syntax parser rejects is reported as an error.

Unreadable files, unknown languages, and syntax errors stop the command before any files are written. A write failure can leave earlier files updated.
