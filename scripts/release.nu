def main []: nothing -> string {
    help main
}

def "main plan" []: nothing -> string {
    let configuration = open dist-workspace.toml | get dist
    let version = dist --version | str trim | split row --regex \s+ | last

    if $version != $configuration.cargo-dist-version {
        error make {
            msg: $"dist version ($version) does not match ($configuration.cargo-dist-version)"

            labels: [
                {
                    text: "installed version"
                    span: (metadata $version).span
                }
            ]
        }
    }

    let matrix = {
        include: ($configuration.targets | each {|target|
            let runner = match $target {
                aarch64-apple-darwin => "macos-26"
                aarch64-unknown-linux-gnu => "ubuntu-26.04-arm"
                x86_64-apple-darwin => "macos-26-intel"
                x86_64-unknown-linux-gnu => "ubuntu-26.04"
                x86_64-pc-windows-msvc => "windows-2025"

                _ => {
                    error make {
                        msg: $"unsupported release target: ($target)"

                        labels: [
                            {
                                text: "unsupported target"
                                span: (metadata $target).span
                            }
                        ]
                    }
                }
            }

            {target: $target, runner: $runner}
        })
    }

    $"matrix=($matrix | to json --raw)\n" | save --append $env.GITHUB_OUTPUT

    let arguments = if $env.GITHUB_EVENT_NAME == push {
        [--tag $env.GITHUB_REF_NAME]
    } else {
        []
    }

    dist plan ...$arguments --output-format=json
}

def "main archive" []: nothing -> nothing {
    let manifest = dist build --artifacts=local --target $env.TARGET --tag $env.GITHUB_REF_NAME --output-format=json | from json
    $manifest | to json | save --force dist-manifest.json
    let destination = $"target/distrib/($env.TARGET)-dist-manifest.json"
    cp dist-manifest.json $destination
    let paths = $manifest.upload_files | append $destination | str join (char newline)
    $"paths<<EOF\n($paths)\nEOF\n" | save --append $env.GITHUB_OUTPUT
}

def "main package" []: nothing -> nothing {
    dist build --artifacts=global --tag $env.GITHUB_REF_NAME --output-format=json
    let manifest = dist manifest --artifacts=all --tag $env.GITHUB_REF_NAME --output-format=json | from json
    $manifest | to json | save --force dist-manifest.json
}

def "main publish" []: nothing -> string {
    let manifest = open dist-manifest.json
    let notes = $env.RUNNER_TEMP | path join notes.md
    $manifest.announcement_github_body | save --force $notes

    let arguments = if $manifest.announcement_is_prerelease {
        [--prerelease]
    } else {
        []
    }

    let artifacts = $manifest.upload_files | append dist-manifest.json
    gh release create $env.GITHUB_REF_NAME --verify-tag --title $env.GITHUB_REF_NAME --notes-file $notes ...$arguments ...$artifacts
}
