# cspot: C bindings for librespot

<!-- PROJECT SHIELDS -->
[![Build Status][build-shield]][build-url]
[![Issues][issues-shield]][issues-url]
[![MIT License][license-shield]][license-url]

cspot provides C bindings for the [librespot](https://github.com/librespot-org/librespot) library so C and C++ applications can act as Spotify Connect clients. The project exposes a small C API, ships a generated `cspot.h`, and includes sample applications that link against the bindings.

librespot is included as a git submodule.

## What’s in this repo

- `c-bindings/`: the Rust crate that exports the C ABI, plus cbindgen config for `cspot.h`.
- `samples/`: C examples that link against the cspot library.
- `artifacts/`: build outputs (configured by CMake presets).
- `scripts/setup/`: OS-specific setup scripts for containers, workstations, and CI agents.

## Setup scripts

From the repository root directory, run the following command depending on your platform to bootstrap a new machine (or container) with the required toolchains and native libraries:

Linux or macOS:

```sh
./scripts/setup.sh
```

Windows:

```bat
.\scripts\setup.cmd
```

The scripts install Rust (via rustup), cbindgen, CMake, Ninja, and a compiler toolchain. On Linux they also install minimal librespot native deps (OpenSSL + ALSA for the rodio backend). See `scripts/setup/README.md` for details.

To bootstrap Android toolchains (SDK command-line tools, NDK, and Rust Android targets), add `--android`:

```sh
./scripts/setup.sh --android
```

## Build from Source

cspot ships an extensive `CMakePresets.json` that encapsulates the common configure, build, test, and package workflows. Presets stage outputs in `artifacts/` so libraries, headers, and sample binaries stay organized.

### Get the source

Clone the repo and initialize the librespot submodule:

```sh
git clone --recurse-submodules https://github.com/MJRasicci/cspot.git
```

If you already cloned without submodules:

```sh
git submodule update --init --recursive
```

### Using CMake presets

Presets follow a `{platform}-{architecture}-{configuration}` convention (for example `linux-x64-debug` or `windows-arm64-release`). Each platform preset inherits shared base settings so you can mix and match:

- **Platform:** `windows`, `macos`, `linux`, `android`
- **Architecture:** desktop presets use `x64`/`arm64`; Android presets use `x86`, `x86_64`, `armeabi-v7a`, `arm64-v8a`
- **Configuration:** `debug`, `release`

Configure with your preferred preset:

```sh
cmake --preset linux-x64-debug
```

Build targets for the active preset:

```sh
cmake --build --preset linux-x64-debug
```

Android builds require `ANDROID_NDK_HOME` in the environment (set by `./scripts/setup.sh --android` in the current shell).

### Running tests

If tests are present, run them via CTest:

```sh
ctest --preset linux-x64-debug
```

### Packaging

Packaging presets are provided for **release** configurations only. Use a release preset to produce distributable bundles:

```sh
cmake --build --preset linux-x64-release --target package
```

Packages contain:

- `include/`: `cspot.h`
- `lib/`: static library artifacts
- `samples/`: sample executables

Android package presets place libraries under `lib/<abi>/`.

### Workflow shortcuts

Workflow presets chain multiple steps together. Debug workflows run **configure → build → test**. Release workflows add **package**:

```sh
cmake --workflow --preset linux-x64-debug     # configure + build + test
cmake --workflow --preset linux-x64-release   # configure + build + test + package
cmake --workflow --preset android-arm64-v8a-release
```

Run all Android ABI workflows individually:

```sh
cmake --workflow --preset android-x86-release
cmake --workflow --preset android-x86_64-release
cmake --workflow --preset android-armeabi-v7a-release
cmake --workflow --preset android-arm64-v8a-release
```

Build and package all Android ABIs into one consolidated archive:

```sh
cmake --workflow --preset android-all-release
```

This produces `artifacts/pack/cspot-android-all-release.tar.gz` with:
- `include/cspot.h`
- `lib/x86/libcspot.{so,a}`
- `lib/x86_64/libcspot.{so,a}`
- `lib/armeabi-v7a/libcspot.{so,a}`
- `lib/arm64-v8a/libcspot.{so,a}`

### Install to a prefix

```sh
cmake --install --preset linux-x64-debug
```

The install tree mirrors the package layout.

## Notes

- CMake presets are defined in `CMakePresets.json` at the repo root.
- Build outputs go under `artifacts/` by default.
- The cspot crate enables librespot's `rodio-backend` by default; disable it or swap backends via Cargo features if you need a different audio output path.
- If you need a different compiler or generator, add a new preset instead of editing build scripts.

[build-shield]: https://img.shields.io/github/actions/workflow/status/mjrasicci/cspot/build.yml?branch=main&logo=github&style=for-the-badge
[build-url]: https://github.com/mjrasicci/cspot/actions/workflows/build.yml
[issues-shield]: https://img.shields.io/github/issues/mjrasicci/cspot.svg?logo=github&style=for-the-badge
[issues-url]: https://github.com/mjrasicci/cspot/issues
[license-shield]: https://img.shields.io/github/license/mjrasicci/cspot.svg?style=for-the-badge
[license-url]: https://github.com/mjrasicci/cspot/blob/main/LICENSE.txt
