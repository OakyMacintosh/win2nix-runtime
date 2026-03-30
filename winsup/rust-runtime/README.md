# win2nix Rust runtime (Win32 + Unix)

This directory is an in-progress Rust migration layer for the MSYS2/Cygwin runtime codebase.

## Implemented now

- WoA-focused architecture detection with `IsWow64Process2` on Windows.
- High-resolution time implementations:
  - Windows: `QueryPerformanceCounter`, `GetSystemTimePreciseAsFileTime`
  - Unix: `clock_gettime(CLOCK_MONOTONIC/CLOCK_REALTIME)`
- Cross-platform process ID retrieval.
- Cross-platform readonly file open helper.
- Optional C ABI exports via the `ffi` cargo feature.
- Initial Rust port of MSYS2 path classification and path conversion helpers in `src/msys2.rs`.
- Initial Rust port of `textreadmode.c` entrypoint in `src/textreadmode.rs` (Windows-only module).

## Conversion status

A full conversion of the historical `winsup/cygwin` C/C++ codebase is very large and is not complete in this change.
This crate provides the foundation plus concrete Rust ports for key MSYS2 runtime behaviors so additional runtime files can be moved incrementally while preserving ABI compatibility.

## Build

```bash
cargo test
cargo test --features ffi
```

