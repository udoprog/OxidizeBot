.\tools\env.ps1

function Msi-Version($maj, $min, $patch, $pre) {
    <#
    Calculate an MSI-safe version number.
    Unfortunately this enforces some unfortunate constraints on the available
    version range.

    The computed patch component must fit within 65535
    #>

    if ([int]$patch -gt 64) {
        throw "Patch version must not be greater than 64: $patch"
    }

    if ([int]$pre -ge 999) {
        throw "Prerelease version must not be greater than 998: $pre"
    }

    if (!$pre) {
        $last = 999
    } else {
        $last = [int]$pre
    }

    $last += [int]$patch * 1000;
    "$maj.$min.$last"
}

Function Run-Cargo([string[]]$Arguments) {
    Write-Host "cargo $Arguments"
    cargo $Arguments
    $LastExitCode -eq 0
}

Function Run-SignTool([string[]]$Arguments) {
    Write-Host "signtool $Arguments"
    signtool $Arguments
    $LastExitCode -eq 0
}

Function Sign($Root, $File, $What) {
    Write-Host "Signing $file"
    return Run-SignTool "sign","/f","$Root/bot/res/cert.pfx","/d","$what","/du","https://github.com/udoprog/OxidizeBot","/p",$env:CERTIFICATE_PASSWORD,$file
}

if (!($env:APPVEYOR_REPO_TAG_NAME -match '^(\d+)\.(\d+)\.(\d+)(-.+\.(\d+))?$')) {
    Write-Output "Testing..."

    if (!(Run-Cargo "build","--all")) {
        throw "Build failed"
    }

    if (!(Run-Cargo "test","--all")) {
        throw "Tests failed"
    }

    exit
}

$root="$PSScriptRoot/.."
$version=$env:APPVEYOR_REPO_TAG_NAME

if (!(Run-Cargo "build","--release","--bin","oxidize")) {
    throw "Failed to build binary"
}

if (Test-Path env:CERTIFICATE_PASSWORD) {
    Sign -Root $root -File "$root/target/release/oxidize.exe" -What "OxidizeBot"
}

if (!(Test-Path $root/target/wix)) {
    $msi_version=Msi-Version $Matches[1] $Matches[2] $Matches[3] $Matches[5]

    if (!(Run-Cargo "wix","-n","oxidize","--install-version",$msi_version,"--nocapture")) {
        throw "Failed to build wix package"
    }
}

$installers = Get-ChildItem -Path $root/target/wix -Include *.msi -Recurse

if (Test-Path env:CERTIFICATE_PASSWORD) {
    foreach ($file in $installers) {
        Sign -Root $root -File $file -What "OxidizeBot Installer"
    }
}

$installers | Copy-Item -Destination $root

$zip="oxidize-$version-windows-x86_64.zip"
7z a $zip $root/README.md
7z a $zip $root/target/release/oxidize.exe