if ($env:APPVEYOR_REPO_TAG_NAME -match '^(\d+\.\d+)\.\d+$') {
    $release = $matches[1]
    $version = $env:APPVEYOR_REPO_TAG_NAME
} elseif ($env:APPVEYOR_REPO_BRANCH -match '^point-(\d+\.\d+)$') {
    $release = $matches[1]
    $version=Get-Date -UFormat '%Y%m%d-%H%M%S'
} else {
    Write-Output "Testing..."
    & cmd /c 'cargo build --all 2>&1'
    & cmd /c 'cargo test --all 2>&1'
    exit
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