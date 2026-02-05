# cspot Agent Guidance

This repo builds **cspot**, a C ABI wrapper around the Rust **librespot** library (Spotify Connect client). The goal is a clean, well-documented C interface, stable packaging, and reliable CMake workflows.

## Expectations

- **Quality bar:** high. Prefer correct, explicit, testable solutions over quick patches.
- **Documentation:** public functions, structs, and modules must have doc comments explaining intent, usage, and invariants. If you add a public API, document it.
- **Organization:** keep related types and functions grouped in dedicated files/modules. Avoid monolithic files. Separate concerns (FFI boundary, core logic, utilities, build/packaging helpers).
- **Safety:** minimize unsafe Rust. When needed, justify it with comments and keep the unsafe surface small.
- **Consistency:** follow existing style and conventions. Keep APIs coherent and predictable.

## Code structure guidance

- Put FFI surface in `c-bindings/` and keep the Rust internals separate from exported symbols.
- Prefer small, focused modules over large files. If a file grows beyond a few hundred lines, split it.
- Keep C-facing types simple and stable; add conversion helpers inside Rust.
- Clearly document ownership, lifetimes, and threading expectations for any FFI handle.

## Build & packaging expectations

- CMake is the primary workflow. Presets and workflows should remain first-class. You can build the project using `cmake --workflow --preset {os}-{arch}-debug` where os is windows, macos, or linux and arch is x64 or arm64. This will run cbindgen to generate the cspot header, cargo to build libcspot, and your C compiler to build the sample projects.
- Build outputs must stay under `artifacts/`.
- Packaging should produce `include/`, `lib/`, and `samples/` in the final bundle.
- You should always build the project using the cmake workflow after any code changes to ensure your output compiles.

## Prompting tips for future changes

When proposing code, include:

- What files you will touch and why.
- Any public API changes and their rationale.
- Safety or lifetime considerations for FFI.
- Tests or verification steps (even if not run).
- If unsure about intent, ask clarifying questions before coding.

## Review checklist (before final response)

- Are new APIs documented and organized?
- Are unsafe blocks minimal and justified?
- Are build steps consistent with CMake presets/workflows?
- Are samples and packaging still working?
