# win2nix Rust runtime (Win32 + Unix)

This directory introduces an incremental Rust runtime layer meant to replace C runtime utility code over time while preserving C ABI compatibility.

## What is included

- WoA-focused architecture detection with `IsWow64Process2` on Windows.
- High-resolution time implementations:
  - Windows: `QueryPerformanceCounter`, `GetSystemTimePreciseAsFileTime`
  - Unix: `clock_gettime(CLOCK_MONOTONIC/CLOCK_REALTIME)`
- Cross-platform process ID retrieval.
- Cross-platform readonly file open helper.
- Optional C ABI exports via the `ffi` cargo feature.

## Why this is useful for Windows on ARM

On Snapdragon/Windows on ARM machines, runtime behavior can depend on host CPU architecture and emulation state. The `detect_capabilities` API records:

- host architecture (`x86`, `x86_64`, `arm64`)
- process architecture (native/emulated)
- expected x64/x86 emulation availability

This allows upstream runtime code to make architecture-aware decisions without relying on ad-hoc C preprocessor logic.

## Build

```bash
cargo test
cargo test --features ffi
```

