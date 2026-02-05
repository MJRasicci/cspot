# cspot: C bindings for librespot

cspot provides C bindings for the [librespot](https://github.com/librespot-org/librespot) library so C and C++ applications can act as Spotify Connect clients. The project exposes a small C API, ships a generated `cspot.h`, and includes sample applications that link against the bindings.

librespot is included as a git submodule.

## What’s in this repo

- `c-bindings/`: the Rust crate that exports the C ABI, plus cbindgen config for `cspot.h`.
- `samples/`: C examples that link against the cspot library.
- `artifacts/`: build outputs (configured by CMake presets).

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

- **Platform:** `windows`, `macos`, `linux`
- **Architecture:** `x64`, `arm64`
- **Configuration:** `debug`, `release`

Configure with your preferred preset:

```sh
cmake --preset linux-x64-debug
```

Build targets for the active preset:

```sh
cmake --build --preset linux-x64-debug
```

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

### Workflow shortcuts

Workflow presets chain multiple steps together. Debug workflows run **configure → build → test**. Release workflows add **package**:

```sh
cmake --workflow --preset linux-x64-debug     # configure + build + test
cmake --workflow --preset linux-x64-release   # configure + build + test + package
```

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
