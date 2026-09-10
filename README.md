# breathers

Give Rust code some breathing room. **breathers** plays on *breathers* and *breathe-rs*: breathe Rust.

Automatically add blank lines around control flow, blocks, and multiline statements. Keep consecutive simple statements together.

## Install

```sh
cargo install --git https://github.com/0hirume/breathers --locked
```

## Usage

Run your usual Rust formatter, then pass the files you want to space:

```sh
breathers src/main.rs
breathers src/main.rs src/lib.rs
```

Files are edited in place. Pass file paths, not directories.

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
