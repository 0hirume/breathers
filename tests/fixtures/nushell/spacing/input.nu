def example [] {
    let ready = true
    # attached
    if $ready {
        print ready
    }
    ls
    | where size > 0
    | get name
    return done
}
