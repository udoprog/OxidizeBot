param (
    [string]$release,
    [string]$version
)

if (!$version) {
    $version = $env:APPVEYOR_REPO_TAG_NAME
}

if (!$version) {
    Write-Output "Testing..."
    & cmd /c 'cargo build --all 2>&1'
    & cmd /c 'cargo test --all 2>&1'
    exit
}

if (!$release) {
    if (!($version -match '^(\d+)\.(\d+)\.\d+$')) {
        throw "bad version: $version"
    }

    $maj = $matches[1]
    $min = $matches[2]
    $release = "$maj.$min"
}

& cmd /c 'cargo build --release --bin setmod-bot 2>&1'

$dest="setmod-$release"
$target="target/$dest"

if (Test-Path -Path $target) {
    Remove-Item -Recurse -Force $target
}

New-Item -Name $target -ItemType "directory"

# example secrets.yml
Copy-Item log4rs.yaml -Destination $target/
Copy-Item secrets.yml.example -Destination $target/
Copy-Item config.toml.example -Destination $target/
Copy-Item target/release/setmod-bot.exe -Destination $target/
Copy-Item tools/setmod-dist.ps1 -Destination $target/setmod.ps1

Set-Location -Path target
7z a "setmod-$version-windows-x86_64.zip" $dest/