# cspot setup scripts

This directory contains OS-specific setup helpers for containers, new workstations, and CI agents.

## What gets installed

- Rust toolchain via rustup (plus rustfmt and clippy)
- cbindgen
- CMake and Ninja
- A C/C++ compiler toolchain
- Minimal librespot native dependencies (native-tls + rodio backend)

## Usage

From the repository root directory, run the following command depending on your platform.

Linux or macOS:

```sh
./scripts/setup.sh
```
Windows:

```bat
.\scripts\setup.cmd
```

### Android toolchain bootstrap

Pass `--android` to install Android SDK command-line tools, platform/build-tools packages,
the Android NDK, and Rust Android targets (`x86`, `x86_64`, `armeabi-v7a`, `arm64-v8a`):

Linux or macOS:

```sh
./scripts/setup.sh --android
```

Windows:

```bat
.\scripts\setup.cmd --android
```

## Notes

- macOS: the Xcode Command Line Tools install is interactive; re-run the script after it completes.
- Windows: `winget` is preferred, with `choco` as a fallback if already installed.
- The scripts update git submodules when run inside a git checkout.
- Linux installs ALSA dev headers to support the rodio backend; swap packages if you want a different audio backend.
- Android setup defaults can be overridden with `CSPOT_ANDROID_API_LEVEL`, `CSPOT_ANDROID_BUILD_TOOLS_VERSION`, `CSPOT_ANDROID_NDK_VERSION`, and `CSPOT_ANDROID_CMDLINE_TOOLS_URL`.
- To produce one archive containing all Android ABIs after setup, run `cmake --workflow --preset android-all-release`.
