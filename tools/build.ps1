.\tools\env.ps1

function Msi-Version($maj, $min, $patch, $pre) {
    <#
    Calculate an MSI-safe version number.
    Unfortunately this enforces some unfortunate constraints on the available
    version range.

    The computed patch component must fit within 65535
    #>

    if ([int]$patch -gt 65) {
        throw "Patch version must not be greater than 65: $patch"
    }

    $patch=[string]$patch

    if ($pre) {
        if ([int]$pre -gt 534) {
            throw "Pre-release version must not be greater than 534: $pre"
        }

        $pre=$pre.PadLeft(3, '0')
    } else {
        $pre = "535"
    }

    "$maj.$min.$patch$pre"
}

Function Run-Cargo([string[]]$Arguments) {
    Write-Host "cargo $Arguments"
    cargo $Arguments
    $LastExitCode -eq 0
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

$version=$env:APPVEYOR_REPO_TAG_NAME
$msi_version=Msi-Version $Matches[1] $Matches[2] $Matches[3] $Matches[5]

if (!(Run-Cargo "build","--release","--bin","setmod")) {
    throw "Failed to build binary"
}

if (!(Run-Cargo "wix","-n","setmod","--install-version",$msi_version,"--nocapture")) {
    throw "Failed to build wix package"
}

$root="$PSScriptRoot/.."
$zip="setmod-$version-windows-x86_64.zip"

Get-ChildItem -Path $root/target/wix -Include *.msi -Recurse | Copy-Item -Destination $root

7z a $zip $root/log4rs.yaml
7z a $zip $root/target/release/setmod.exe
7z a $zip $root/build/dll/*.dll