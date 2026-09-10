# breathers

Give Rust code some breathing room. **breathers** plays on *breathers* and *breathe-rs*: breathe Rust.

Automatically add blank lines around control flow, blocks, and multiline statements. Keep consecutive simple statements together.

## Install

```sh
cargo install --git https://github.com/0hirume/breathers --locked
```

## Usage

Run your usual Rust formatter, then choose what to space:

```sh
breathers                         # All Rust files under the current directory
breathers src/                    # All Rust files under src
breathers src/main.rs              # Only src/main.rs
breathers src/main.rs src/lib.rs   # Multiple files
```

Files are edited in place. Directory searches include nested directories and select `.rs` files, skipping `.git`, `target`, and symbolic links. Overlapping paths are processed once. Explicit file paths are processed directly.

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

- Adds space around `if`, `match`, loops, block expressions, and multiline statements.
- Keeps simple statements together and leaves block edges alone.
- Preserves existing blank lines, comment text, strings, and line endings.
- Leaves compact single-line code and macro contents alone.
- Repeated runs leave the result unchanged.

Unreadable files and syntax errors stop the command before any files are written. A write failure can leave earlier files updated.
