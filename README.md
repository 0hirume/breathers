# breathers

breathers adds blank lines around control flow, blocks, and multiline statements. It keeps consecutive simple statements together and preserves existing blank lines, comments, strings, and line endings.

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

Run your usual formatter first. breathers edits files in place and searches directories recursively. If you omit paths, it uses the current directory. By default, it reports changed files, errors, and a summary on stderr.

```sh
breathers
breathers src/
breathers src/main.rs src/main.luau
```

| Option                        | Short | Behavior                                                                             |
| ----------------------------- | ----- | ------------------------------------------------------------------------------------ |
| `--check`                     | `-k`  | Check without writing; return a nonzero exit code for needed changes or errors       |
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

Use at most one of `--check`, `--preview`, and `--source`. `--source` requires exactly one selected file and does not support `--json`. Do not combine `--quiet` and `--verbose`. `--quiet` still prints output you request on stdout.

```sh
breathers -k src/
breathers -p src/main.rs
breathers -s src/main.rs
breathers -q -i 'src/**/*.rs' -e 'src/generated/**'
```

JSON reports include the mode, file statuses, summary counts, and errors. Preview reports include diffs. Stdin reports include formatted source. breathers also prints errors to stderr and returns a nonzero exit code on failure.

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

Override detection with `-l` or `--language`. Use `-l luau` for Luau files named `.lua`, or `-l tsx` to select TSX.

```sh
breathers -l luau src/module.lua
```

To format stdin, use `-` as the only path. Set its language with `-l`, or use `-f` to supply a filepath. breathers prints the result to stdout and does not write to that filepath.

```sh
breathers -l luau -
breathers -f src/module.luau -
```

## C and C++

C and C++ require LLVM's `libclang` shared library. Install it and set `LIBCLANG_PATH` to its directory if breathers cannot find it.

breathers reads compiler flags from the nearest `compile_commands.json`. It also reads `.clangd` files in parent directories, which can add flags with `CompileFlags.Add`. Use `If.PathMatch` and `If.PathExclude` to select which files receive those flags. breathers rejects other compiler-flag settings.

For missing headers or declarations, supply your project's include paths and defines in those files. C compiler errors leave the file unchanged. C++ also accepts syntactically valid snippets with unresolved names. Inactive branches and macro contents stay unchanged.

## Configuration

breathers loads global settings, then applies the nearest `breathers.toml` above the working directory. Use `-c` or `--config` to select a project configuration.

Global settings use `breathers/config.toml` under `%APPDATA%` on Windows or `~/Library/Application Support` on macOS. On Linux, the base directory is `$XDG_CONFIG_HOME` or, if unset, `~/.config`.

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

Globs use paths relative to the project configuration directory. If breathers finds no project configuration, it uses the working directory. Command-line includes replace configured includes. Command-line exclusions add to configured exclusions and take precedence over includes. Directory searches skip `.git` and symbolic links.

Language sections control spacing. JSX uses `javascript`. TSX uses `typescript`. breathers enables rule groups by default. Individual rules inherit their group's setting unless you override it. The `related` rule is off by default. Enable it to keep adjacent variable dependencies together. It preserves existing blank lines.

See [`schema.json`](schema.json) for all language sections and options, or run `breathers --schema`.

## Request a language

[Open an issue](https://github.com/0hirume/breathers/issues/new) with the language and a before/after example showing the spacing you want.
