def main []: nothing -> string {
    try { cargo fmt --check } catch {|failure| exit ($failure.exit_code? | default 1) }

    main verify
}

def "main verify" []: nothing -> string {
    try { cargo clippy --all-targets --locked } catch {|failure| exit ($failure.exit_code? | default 1) }

    try { cargo test --test fixtures --locked } catch {|failure| exit ($failure.exit_code? | default 1) }
}
