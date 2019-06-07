.\tools\env.ps1

if ($env:APPVEYOR_REPO_TAG_NAME -match '^\d+\.\d+\.\d+$') {
    $version = $env:APPVEYOR_REPO_TAG_NAME
} elseif ($env:APPVEYOR_REPO_BRANCH -match '^point-\d+\.\d+$') {
    $version=Get-Date -UFormat '%Y%m%d-%H%M%S'
} else {
    Write-Output "Testing..."
    & cmd /c 'cargo build --all 2>&1'
    & cmd /c 'cargo test --all 2>&1'
    exit
}

& cmd /c 'cargo build --release --bin setmod 2>&1'
& cmd /c "cargo wix -n setmod --install-version $version 2>&1"

$root="$PSScriptRoot/.."
$zip="setmod-$version-windows-x86_64.zip"

Get-ChildItem -Path $root/target/wix -Include *.msi -Recurse | Copy-Item -Destination $root

7z a $zip $root/log4rs.yaml
7z a $zip $root/target/release/setmod.exe
7z a $zip $root/build/dll/*.dll