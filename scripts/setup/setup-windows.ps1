param(
  [switch]$Android,
  [switch]$Help
)

$ErrorActionPreference = "Stop"
$ProgressPreference = "SilentlyContinue"

$AndroidApiLevel = if ($env:CSPOT_ANDROID_API_LEVEL) { $env:CSPOT_ANDROID_API_LEVEL } else { "26" }
$AndroidBuildToolsVersion = if ($env:CSPOT_ANDROID_BUILD_TOOLS_VERSION) { $env:CSPOT_ANDROID_BUILD_TOOLS_VERSION } else { "34.0.0" }
$AndroidNdkVersion = if ($env:CSPOT_ANDROID_NDK_VERSION) { $env:CSPOT_ANDROID_NDK_VERSION } else { "27.2.12479018" }
$AndroidCmdlineToolsVersion = if ($env:CSPOT_ANDROID_CMDLINE_TOOLS_VERSION) { $env:CSPOT_ANDROID_CMDLINE_TOOLS_VERSION } else { "13114758" }

function Show-Usage {
  Write-Host @"
Usage: .\scripts\setup.cmd [--android]

Options:
  --android   Install Android SDK/NDK tooling and Rust Android targets.
  --help      Show this help text.
"@
}

if ($Help) {
  Show-Usage
  exit 0
}

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
  param([switch]$IncludeAndroid)

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
    if ($IncludeAndroid) {
      Use-Winget -Id "EclipseAdoptium.Temurin.17.JDK"
    }
    return
  }

  Write-Log "Installing system dependencies via chocolatey"
  Use-Choco -Name "visualstudio2022buildtools" -ExtraArgs @(
    "--package-parameters=\"--add Microsoft.VisualStudio.Workload.VCTools --includeRecommended --passive --norestart\""
  )
  Use-Choco -Name "cmake"
  Use-Choco -Name "ninja"
  Use-Choco -Name "llvm"
  if ($IncludeAndroid) {
    Use-Choco -Name "temurin17"
  }
}

function Resolve-AndroidSdkRoot {
  if ($env:ANDROID_SDK_ROOT) {
    return $env:ANDROID_SDK_ROOT
  }
  if ($env:ANDROID_HOME) {
    return $env:ANDROID_HOME
  }
  return (Join-Path $env:LOCALAPPDATA "Android\Sdk")
}

function Set-UserEnvVar {
  param(
    [string]$Name,
    [string]$Value
  )

  [Environment]::SetEnvironmentVariable($Name, $Value, "User")
  Set-Item -Path "Env:$Name" -Value $Value
}

function Ensure-AndroidCommandlineTools {
  param([string]$SdkRoot)

  $sdkManager = Join-Path $SdkRoot "cmdline-tools\latest\bin\sdkmanager.bat"
  if (Test-Path $sdkManager) {
    return
  }

  $archiveUrl = if ($env:CSPOT_ANDROID_CMDLINE_TOOLS_URL) {
    $env:CSPOT_ANDROID_CMDLINE_TOOLS_URL
  } else {
    "https://dl.google.com/android/repository/commandlinetools-win-$AndroidCmdlineToolsVersion" + "_latest.zip"
  }

  $tmpDir = Join-Path $env:TEMP ("cspot-android-" + [Guid]::NewGuid().ToString("N"))
  $zipPath = Join-Path $tmpDir "commandlinetools.zip"
  $extractRoot = Join-Path $tmpDir "extract"
  $cmdlineToolsRoot = Join-Path $SdkRoot "cmdline-tools"
  $latestRoot = Join-Path $cmdlineToolsRoot "latest"

  Write-Log "Installing Android command-line tools"
  New-Item -ItemType Directory -Path $tmpDir -Force | Out-Null
  New-Item -ItemType Directory -Path $extractRoot -Force | Out-Null
  New-Item -ItemType Directory -Path $cmdlineToolsRoot -Force | Out-Null

  try {
    Invoke-WebRequest -Uri $archiveUrl -OutFile $zipPath
    Expand-Archive -Path $zipPath -DestinationPath $extractRoot -Force

    $extractedRoot = Join-Path $extractRoot "cmdline-tools"
    if (-not (Test-Path $extractedRoot)) {
      throw "Android command-line tools archive did not contain cmdline-tools/"
    }

    if (Test-Path $latestRoot) {
      Remove-Item -Path $latestRoot -Recurse -Force
    }
    Move-Item -Path $extractedRoot -Destination $latestRoot
  } finally {
    if (Test-Path $tmpDir) {
      Remove-Item -Path $tmpDir -Recurse -Force
    }
  }
}

function Ensure-AndroidSdkComponents {
  param([string]$SdkRoot)

  $sdkManager = Join-Path $SdkRoot "cmdline-tools\latest\bin\sdkmanager.bat"
  if (-not (Test-Path $sdkManager)) {
    throw "sdkmanager was not found at $sdkManager"
  }

  $env:Path = "$SdkRoot\cmdline-tools\latest\bin;$SdkRoot\platform-tools;$env:Path"

  Write-Log "Accepting Android SDK licenses"
  "y`ny`ny`n" | & $sdkManager "--sdk_root=$SdkRoot" "--licenses" *> $null

  Write-Log "Installing Android SDK platform, build-tools, and NDK"
  $packages = @(
    "platform-tools",
    "platforms;android-$AndroidApiLevel",
    "build-tools;$AndroidBuildToolsVersion",
    "ndk;$AndroidNdkVersion"
  )
  & $sdkManager "--sdk_root=$SdkRoot" @packages
}

function Ensure-AndroidRustTargets {
  if (-not (Have-Command "rustup")) {
    throw "rustup is required to add Android Rust targets"
  }

  Write-Log "Installing Rust Android targets"
  & rustup target add `
    aarch64-linux-android `
    armv7-linux-androideabi `
    i686-linux-android `
    x86_64-linux-android
}

function Ensure-AndroidToolchain {
  $sdkRoot = Resolve-AndroidSdkRoot
  New-Item -ItemType Directory -Path $sdkRoot -Force | Out-Null

  Set-UserEnvVar -Name "ANDROID_SDK_ROOT" -Value $sdkRoot
  Set-UserEnvVar -Name "ANDROID_HOME" -Value $sdkRoot

  Ensure-AndroidCommandlineTools -SdkRoot $sdkRoot
  Ensure-AndroidSdkComponents -SdkRoot $sdkRoot

  $ndkHome = Join-Path $sdkRoot "ndk\$AndroidNdkVersion"
  if (-not (Test-Path $ndkHome)) {
    throw "failed to install Android NDK $AndroidNdkVersion"
  }

  Set-UserEnvVar -Name "ANDROID_NDK_HOME" -Value $ndkHome
  Set-UserEnvVar -Name "ANDROID_NDK_ROOT" -Value $ndkHome
  Ensure-AndroidRustTargets

  Write-Log "Android SDK root: $sdkRoot"
  Write-Log "Android NDK home: $ndkHome"
  Write-Warn "Open a new PowerShell session to pick up persisted Android environment variables."
}

function Update-Submodules {
  $repoRoot = Resolve-Path (Join-Path $PSScriptRoot "..\..")
  if ((Have-Command "git") -and (Test-Path (Join-Path $repoRoot ".git"))) {
    Write-Log "Updating git submodules"
    & git -C $repoRoot submodule update --init --recursive
  }
}

function Main {
  Ensure-SystemDependencies -IncludeAndroid:$Android
  Ensure-Rustup
  Ensure-RustComponents
  Ensure-Cbindgen
  if ($Android) {
    Ensure-AndroidToolchain
  }
  Update-Submodules

  Write-Log "Setup complete. Open a new PowerShell session to pick up PATH updates if needed."
}

Main
