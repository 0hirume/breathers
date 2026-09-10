# breathers

Give Rust code some breathing room. **breathers** plays on *breathers* and *breathe-rs*: breathe Rust.

The `breather` binary inserts blank lines around control flow, blocks, and multiline statements while keeping consecutive simple statements together. It uses rust-analyzer's syntax parser and only inserts whitespace.

## Usage

From this checkout, run rustfmt first, then pass explicit file paths:

```sh
cargo fmt
cargo run -- src/main.rs
```

Multiple files are accepted:

```sh
cargo run -- src/main.rs src/lib.rs
```

Files are edited in place. Unchanged files are not written. All inputs are read and parsed before writing begins; a read or syntax error prevents any writes. Writes are sequential, not transactional: a write failure can leave earlier files updated.

## Spacing

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

- Separates neighboring statements when either contains control flow, a block expression, or multiline whitespace.
- Adds spacing only at existing line breaks between statements, including a block's final expression.
- Keeps existing blank lines and leaves block edges alone.
- Preserves comment text, string contents, macro token contents, and existing line endings.
- Repeated runs leave the result unchanged.

## Limits

Pass files explicitly; there is no directory traversal, standard-input mode, or check mode. Compact single-line code stays compact. Macro bodies are not expanded or formatted. Parsing uses the syntax parser's current Rust edition rather than reading Cargo manifests.

## Development

The mise manifest declares nightly Rust with rustfmt and Clippy.

```sh
cargo fmt --check
cargo test --locked
cargo clippy --all-targets --locked
```

Clippy permits duplicate versions of `rustc-hash` in the syntax parser's dependency tree. Other duplicate dependencies remain checked.
