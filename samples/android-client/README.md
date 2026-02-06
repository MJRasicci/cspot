# android-client sample

`android-client` is a minimal Android app sample that links to `libcspot.so` and exposes Spotify Connect controls.

## What it shows

- Connect status and playback state.
- Track metadata: title, artist, album.
- Artwork loaded from the URL reported by cspot.
- Current position, track length, and volume.

## Transport controls

- Previous / next track.
- Play / pause toggle.
- Seek bar for position.
- Volume slider.
- Transfer playback to this device.

## Build integration

This sample is built by Android CMake presets through the CMake target `android_client`.
The target runs a local Gradle build and stages APK outputs at:

- `artifacts/android-client/<abi>/android-client-<abi>-debug.apk`
- `artifacts/android-client/<abi>/android-client-<abi>-release.apk`

Intermediate Gradle and native build outputs are also redirected under:

- `artifacts/android-client/gradle/<abi>/app/build/`
- `artifacts/android-client/gradle/<abi>/app/.cxx/`

## Runtime notes

The app waits for Spotify Connect credentials via discovery. After launching the app,
open Spotify on another device and select the displayed cspot device name.
