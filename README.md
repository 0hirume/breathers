# breathers

breathers adds blank lines around control flow, blocks, and multiline statements.

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

Or download a binary from [GitHub Releases](https://github.com/0hirume/breathers/releases) and place it on your `PATH`.

## Usage

Run your formatter first. breathers edits files in place.

```sh
breathers src/
breathers --check src/
breathers --preview src/main.rs
```

`--check` exits nonzero for needed changes or errors. `--preview` prints a diff.

Supports Rust, Lua, Luau, C, C++, Python, Nushell, JavaScript, JSX, TypeScript, TSX, and PowerShell.

For Luau in `.lua` files, use `--language luau`.

For stdin:

```sh
breathers --stdin-filepath src/main.rs -
```

Run `breathers --help` for all options.

## Configuration

Settings come from the nearest `breathers.toml` above each file. Override with `--config PATH`.

```toml
include = ["src/**"]
exclude = ["generated"]

[luau]
related = true

[rust]
returns = false
```

Patterns use Git-ignore syntax. Directory searches respect `.gitignore`.

`related` keeps adjacent variable dependencies together. See [the schema](https://0hirume.github.io/breathers/schema.json) for all settings.
