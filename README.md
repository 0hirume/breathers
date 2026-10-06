# breathers

Give code some breathing room. **breathers** plays on _breathers_ and _breathe-rs_: breathe Rust.

Add blank lines around control flow, blocks, and multiline statements. Keep consecutive simple statements together and preserve existing blank lines, comments, strings, and line endings.

## Install

With [mise](https://mise.jdx.dev):

```sh
mise use -g github:0hirume/breathers
```

Or download a binary for your platform from [GitHub Releases](https://github.com/0hirume/breathers/releases), extract it, and place it on your `PATH`.

To build from source with Cargo, install Rust and a C/C++ compiler:

```sh
cargo install --git https://github.com/0hirume/breathers --locked
```

## Usage

Run your usual formatter first. Files are edited in place; directories are searched recursively. With no paths, breathers uses the current directory.

```sh
breathers
breathers src/
breathers src/main.rs src/main.luau
```

| Language         | Extensions                                                               |
| ---------------- | ------------------------------------------------------------------------ |
| Rust             | `.rs`                                                                    |
| Lua              | `.lua`                                                                   |
| Luau             | `.luau`                                                                  |
| C                | `.c`, `.h`                                                               |
| C++              | `.cpp`, `.cc`, `.cxx`, `.c++`, `.hpp`, `.hh`, `.hxx`, `.h++`, `.C`, `.H` |
| Python           | `.py`, `.pyi`                                                            |
| Nushell          | `.nu`                                                                    |
| JavaScript / JSX | `.js`, `.jsx`, `.mjs`, `.cjs`                                            |
| TypeScript / TSX | `.ts`, `.mts`, `.cts`, `.tsx`                                            |
| PowerShell       | `.ps1`, `.psm1`, `.psd1`                                                 |

Override detection with `-l` / `--language`. Use `-l luau` for Luau files named `.lua`, or `-l tsx` to select TSX explicitly.

```sh
breathers -l luau src/module.lua
```

For stdin, use `-` as the only path and specify the language. Formatted source goes to stdout:

```sh
breathers -l luau -
```

Start the formatting language server over stdio:

```sh
breathers --lsp
```

## C and C++

C and C++ formatting uses LLVM's `libclang`, loaded at runtime. Make the shared library discoverable or set `LIBCLANG_PATH` to its directory. Other languages do not require it.

Compiler flags come from the nearest ancestor `compile_commands.json`, supporting both `arguments` arrays and quoted `command` strings. Clang selects the source file's command and uses its working directory for relative paths. Ancestor `.clangd` files then add overrides from parent to child. Supported settings are `CompileFlags.Add`, `If.PathMatch`, and `If.PathExclude`, including multiple YAML documents. Conditions match paths relative to their `.clangd` directory; without a compilation command, relative compiler paths use the source file's directory. Other `CompileFlags` settings produce an error. Stdin uses `source.c` for C or `source.cpp` for C++ in the working directory for this discovery; LSP uses the document path and buffer contents.

Clang expands macros and parses active conditional branches. Inactive branches and macro contents are left unchanged. C compiler errors, including missing headers or declarations, are reported without writing changes; supply the project's include paths and defines through its compilation database or `.clangd`.

C++ retains a Tree-sitter syntax-only fallback when Clang reports errors, preserving formatting for snippets with unresolved names. If both parsers reject the source, Clang diagnostics are reported and the file is left unchanged.

## Configuration

Settings come from the nearest `breathers.toml` above the working directory, layered over global settings. For LSP file documents, discovery starts from the document's directory. Use `-c` / `--config` to select a project configuration explicitly.

Global settings live in `breathers/config.toml` under `%APPDATA%` on Windows, `~/Library/Application Support` on macOS, or `$XDG_CONFIG_HOME` (falling back to `~/.config`) on Linux.

```toml
#:schema https://raw.githubusercontent.com/0hirume/breathers/main/schema.json

include = ["src/**"]
exclude = ["**/generated/**"]

[luau]
related = true

[rust]
returns = false
tail_expressions = true
```

Globs are relative to the project configuration directory, or the working directory when no project configuration is found. Exclusions win. Directory searches skip `.git` and symbolic links.

Language sections control spacing. JSX uses `javascript`; TSX uses `typescript`. Broad groups are enabled by default, and individual rules inherit their group unless explicitly set. `related` is opt-in and keeps adjacent variable dependencies together without removing existing blank lines.

See [`schema.json`](schema.json) for all language sections and options, or run `breathers --schema`.

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

## Request a language

[Open an issue](https://github.com/0hirume/breathers/issues/new) with the language and a before/after example showing the spacing you want.
