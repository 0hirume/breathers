# breathers

Give source code breathing room.

Add blank lines around control flow, blocks, and multiline statements. Keep consecutive simple statements together and preserve existing blank lines, comments, strings, and line endings.

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

Run your usual formatter first. Files are edited in place; directories are searched recursively. With no paths, breathers uses the current directory. By default, stderr shows changed files, errors, and one summary.

```sh
breathers
breathers src/
breathers src/main.rs src/main.luau
```

| Option                        | Short | Behavior                                                                             |
| ----------------------------- | ----- | ------------------------------------------------------------------------------------ |
| `--check`                     | `-k`  | Check without writing; exit unsuccessfully when formatting is needed or errors occur |
| `--preview`                   | `-p`  | Print unified diffs to stdout without writing                                        |
| `--source`                    | `-s`  | Print one selected file's formatted source to stdout without writing                 |
| `--quiet`                     | `-q`  | Hide success reports and summary, but keep errors                                    |
| `--verbose`                   | `-v`  | Show progress and unchanged files                                                    |
| `--include PATTERN`           | `-i`  | Replace configured includes; repeat for multiple globs                               |
| `--exclude PATTERN`           | `-e`  | Add exclusions; repeat for multiple globs                                            |
| `--stdin-filepath PATH`       | `-f`  | Give stdin a language, configuration, and compiler context without writing that path |
| `--json`                      | `-j`  | Print a JSON report to stdout                                                        |
| `--color auto\|always\|never` | `-C`  | Control diagnostic colors; automatic colors respect terminals and `NO_COLOR`         |
| `--language LANGUAGE`         | `-l`  | Override language detection                                                          |
| `--config PATH`               | `-c`  | Use an explicit project configuration                                                |
| `--schema`                    | `-S`  | Print the configuration JSON Schema                                                  |
| `--version`                   | `-V`  | Print the version                                                                    |
| `--help`                      | `-h`  | Print help                                                                           |

Check, preview, and source modes are mutually exclusive. Source mode requires exactly one selected file and cannot be combined with JSON. Quiet and verbose are mutually exclusive. Quiet does not suppress requested stdout payloads.

```sh
breathers -k src/
breathers -p src/main.rs
breathers -s src/main.rs
breathers -q -i 'src/**/*.rs' -e 'src/generated/**'
```

JSON reports include the mode, file statuses, summary counts, and errors. Preview reports include diffs; stdin reports include formatted source. Errors also remain visible on stderr, and failures return an unsuccessful exit status.

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

For stdin, use `-` as the only path and specify the language or a context filepath. Formatted source goes to stdout; a context filepath is never a write destination:

```sh
breathers -l luau -
breathers -f src/module.luau -
```

## C and C++

C and C++ require LLVM's `libclang` shared library. Install it and set `LIBCLANG_PATH` to its directory if breathers cannot find it.

breathers reads compiler flags from the nearest `compile_commands.json`. Ancestor `.clangd` files can add flags through `CompileFlags.Add`, optionally filtered by `If.PathMatch` and `If.PathExclude`. Other compiler-flag settings are rejected.

For missing headers or declarations, supply your project's include paths and defines in those files. C compiler errors leave the file unchanged. C++ also accepts syntactically valid snippets with unresolved names. Inactive branches and macro contents stay unchanged.

## Configuration

Settings come from the nearest `breathers.toml` above the working directory, layered over global settings. Use `-c` / `--config` to select a project configuration explicitly.

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

Globs are relative to the project configuration directory, or the working directory when no project configuration is found. CLI includes replace configured includes; CLI exclusions add to configured exclusions. Exclusions win. Directory searches skip `.git` and symbolic links.

Language sections control spacing. JSX uses `javascript`; TSX uses `typescript`. Broad groups are enabled by default, and individual rules inherit their group unless explicitly set. `related` is opt-in and keeps adjacent variable dependencies together without removing existing blank lines.

See [`schema.json`](schema.json) for all language sections and options, or run `breathers --schema`.

## Request a language

[Open an issue](https://github.com/0hirume/breathers/issues/new) with the language and a before/after example showing the spacing you want.
