# This is a helper script to run setmod in a loop, restarting if it shuts down.
# It can be evoked from anywhere on the filesystem.
param([switch] $Release=$false, [string] $WebRoot)

$Args = @()

if ($Release) {
    $Args += "--release"
}

if ($Webroot) {
    $Args += "--"
    $Args += "--web-root"
    $Args += $WebRoot
}

while($true) {
    cargo run --manifest-path="$PSScriptRoot\..\bot\Cargo.toml" $Args
    Write-Host "Bot shut down, restarting in 5s..."
    Start-Sleep -s 5
}