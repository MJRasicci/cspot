$ErrorActionPreference = "Stop"
$ProgressPreference = "SilentlyContinue"

function Write-Log {
  param([string]$Message)
  Write-Host "[cspot-setup] $Message"
}

function Write-Warn {
  param([string]$Message)
  Write-Warning "[cspot-setup] $Message"
}

function Have-Command {
  param([string]$Name)
  return (Get-Command $Name -ErrorAction SilentlyContinue) -ne $null
}

function Ensure-Rustup {
  if (Have-Command "rustc") {
    Write-Log "Rust already installed"
    return
  }

  Write-Log "Installing rustup"
  $rustupInstaller = Join-Path $env:TEMP "rustup-init.exe"
  Invoke-WebRequest -Uri "https://win.rustup.rs" -OutFile $rustupInstaller
  & $rustupInstaller -y

  $cargoBin = Join-Path $env:USERPROFILE ".cargo\bin"
  $env:Path = "$cargoBin;$env:Path"
}

function Ensure-RustComponents {
  if (Have-Command "rustup") {
    & rustup component add rustfmt clippy
  }
}

function Ensure-Cbindgen {
  if (Have-Command "cbindgen") {
    Write-Log "cbindgen already installed"
    return
  }

  if (-not (Have-Command "cargo")) {
    throw "cargo not found; rustup installation may have failed"
  }

  $version = if ($env:CSPOT_CBINDGEN_VERSION) { $env:CSPOT_CBINDGEN_VERSION } else { "0.29.2" }
  Write-Log "Installing cbindgen $version"
  & cargo install cbindgen --version $version
}

function Use-Winget {
  param(
    [string]$Id,
    [string[]]$ExtraArgs = @()
  )
  $args = @(
    "install",
    "--id", $Id,
    "--accept-package-agreements",
    "--accept-source-agreements",
    "--silent"
  ) + $ExtraArgs

  & winget @args
}

function Use-Choco {
  param(
    [string]$Name,
    [string[]]$ExtraArgs = @()
  )
  $args = @("install", "-y", $Name) + $ExtraArgs
  & choco @args
}

function Ensure-SystemDependencies {
  $useWinget = Have-Command "winget"
  $useChoco = Have-Command "choco"

  if (-not $useWinget -and -not $useChoco) {
    throw "winget or chocolatey is required to install dependencies"
  }

  if ($useWinget) {
    Write-Log "Installing system dependencies via winget"
    Use-Winget -Id "Microsoft.VisualStudio.2022.BuildTools" -ExtraArgs @(
      "--override",
      "--add Microsoft.VisualStudio.Workload.VCTools --includeRecommended --passive --norestart"
    )
    Use-Winget -Id "Kitware.CMake"
    Use-Winget -Id "Ninja-build.Ninja"
    Use-Winget -Id "LLVM.LLVM"
    return
  }

  Write-Log "Installing system dependencies via chocolatey"
  Use-Choco -Name "visualstudio2022buildtools" -ExtraArgs @(
    "--package-parameters=\"--add Microsoft.VisualStudio.Workload.VCTools --includeRecommended --passive --norestart\""
  )
  Use-Choco -Name "cmake"
  Use-Choco -Name "ninja"
  Use-Choco -Name "llvm"
}

function Update-Submodules {
  $repoRoot = Resolve-Path (Join-Path $PSScriptRoot "..\..")
  if ((Have-Command "git") -and (Test-Path (Join-Path $repoRoot ".git"))) {
    Write-Log "Updating git submodules"
    & git -C $repoRoot submodule update --init --recursive
  }
}

function Main {
  Ensure-SystemDependencies
  Ensure-Rustup
  Ensure-RustComponents
  Ensure-Cbindgen
  Update-Submodules

  Write-Log "Setup complete. Open a new PowerShell session to pick up PATH updates if needed."
}

Main
