param (
    [Parameter(Mandatory=$true)] [string]$release,
    [Parameter(Mandatory=$true)] [string]$version
)

cargo build --release --bin setmod-bot

$dest="setmod-$release"
$target="target/$dest"

if (Test-Path -Path $target) {
    Remove-Item -Recurse -Force $target
}

New-Item -Name $target -ItemType "directory"

# example secrets.yml
Copy-Item secrets.yml.example -Destination $target/
Copy-Item config.toml.example -Destination $target/
Copy-Item target/release/setmod-bot.exe -Destination $target/
Copy-Item bot/lib/sqlite3.dll -Destination $target/
Copy-Item bot/lib/portaudio_x64.dll -Destination $target/
Copy-Item tools/setmod-dist.ps1 -Destination $target/setmod.ps1

Set-Location -Path target
7z a "setmod-$version.zip" $dest/