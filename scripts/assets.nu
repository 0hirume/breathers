def main []: nothing -> nothing {
    let archive = "breathers-x86_64-unknown-linux-gnu.tar.xz"
    let directory = $env.RUNNER_TEMP | path join breathers
    let output = "target/pages" | path expand

    gh release download $env.RELEASE_TAG --pattern $archive --pattern $"($archive).sha256" --dir $directory

    cd $directory
    sha256sum --check $"($archive).sha256"

    tar --extract --file $archive --strip-components=1

    mkdir $output
    ./breathers --schema | save --raw ($output | path join schema.json)
}
