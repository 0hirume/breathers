def main []: nothing -> string {
    cargo fmt --check
    main verify
}

def "main verify" []: nothing -> string {
    cargo clippy --all-targets --locked
    cargo test --locked
}
