#![deny(unsafe_op_in_unsafe_fn)]

use std::fmt;
use std::path::Path;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuntimeArch {
    X86,
    X64,
    Arm64,
    Unknown,
}

impl fmt::Display for RuntimeArch {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let label = match self {
            RuntimeArch::X86 => "x86",
            RuntimeArch::X64 => "x86_64",
            RuntimeArch::Arm64 => "arm64",
            RuntimeArch::Unknown => "unknown",
        };
        f.write_str(label)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WoaCapabilities {
    pub host_arch: RuntimeArch,
    pub process_arch: RuntimeArch,
    pub x64_emulation_available: bool,
    pub x86_emulation_available: bool,
}

#[cfg(windows)]
mod platform {
    use super::{RuntimeArch, WoaCapabilities};
    use std::ffi::c_void;
    use std::iter;
    use std::os::windows::ffi::OsStrExt;
    use std::path::Path;

    type Handle = *mut c_void;

    const IMAGE_FILE_MACHINE_I386: u16 = 0x014c;
    const IMAGE_FILE_MACHINE_AMD64: u16 = 0x8664;
    const IMAGE_FILE_MACHINE_ARM64: u16 = 0xAA64;

    const GENERIC_READ: u32 = 0x8000_0000;
    const FILE_SHARE_READ: u32 = 0x0000_0001;
    const OPEN_EXISTING: u32 = 3;
    const FILE_ATTRIBUTE_NORMAL: u32 = 0x80;

    const INVALID_HANDLE_VALUE: Handle = -1isize as Handle;

    #[link(name = "kernel32")]
    extern "system" {
        fn GetCurrentProcess() -> Handle;
        fn IsWow64Process2(
            process: Handle,
            process_machine: *mut u16,
            native_machine: *mut u16,
        ) -> i32;
        fn GetCurrentProcessId() -> u32;
        fn GetSystemTimePreciseAsFileTime(file_time: *mut FileTime);
        fn QueryPerformanceCounter(counter: *mut i64) -> i32;
        fn QueryPerformanceFrequency(freq: *mut i64) -> i32;
        fn CreateFileW(
            file_name: *const u16,
            desired_access: u32,
            share_mode: u32,
            security_attributes: *const c_void,
            creation_disposition: u32,
            flags_and_attributes: u32,
            template_file: Handle,
        ) -> Handle;
        fn CloseHandle(handle: Handle) -> i32;
    }

    #[repr(C)]
    struct FileTime {
        low: u32,
        high: u32,
    }

    fn map_machine(machine: u16) -> RuntimeArch {
        match machine {
            IMAGE_FILE_MACHINE_I386 => RuntimeArch::X86,
            IMAGE_FILE_MACHINE_AMD64 => RuntimeArch::X64,
            IMAGE_FILE_MACHINE_ARM64 => RuntimeArch::Arm64,
            _ => RuntimeArch::Unknown,
        }
    }

    pub fn woa_capabilities() -> WoaCapabilities {
        let mut process_machine = 0u16;
        let mut native_machine = 0u16;

        let ok = unsafe {
            IsWow64Process2(
                GetCurrentProcess(),
                &mut process_machine as *mut u16,
                &mut native_machine as *mut u16,
            )
        };

        if ok == 0 {
            return WoaCapabilities {
                host_arch: RuntimeArch::Unknown,
                process_arch: RuntimeArch::Unknown,
                x64_emulation_available: false,
                x86_emulation_available: false,
            };
        }

        let host_arch = map_machine(native_machine);
        let process_arch = if process_machine == 0 {
            host_arch
        } else {
            map_machine(process_machine)
        };

        let x64_emulation_available = host_arch == RuntimeArch::Arm64;
        let x86_emulation_available = host_arch == RuntimeArch::Arm64 || host_arch == RuntimeArch::X64;

        WoaCapabilities {
            host_arch,
            process_arch,
            x64_emulation_available,
            x86_emulation_available,
        }
    }

    pub fn monotonic_time_ns() -> u128 {
        let mut freq = 0i64;
        let mut counter = 0i64;

        let fq_ok = unsafe { QueryPerformanceFrequency(&mut freq) };
        let ct_ok = unsafe { QueryPerformanceCounter(&mut counter) };

        if fq_ok == 0 || ct_ok == 0 || freq <= 0 || counter < 0 {
            return 0;
        }

        (counter as u128 * 1_000_000_000u128) / freq as u128
    }

    pub fn realtime_ns() -> u128 {
        let mut ft = FileTime { low: 0, high: 0 };
        unsafe { GetSystemTimePreciseAsFileTime(&mut ft as *mut FileTime) };

        // FILETIME is in 100ns intervals since Jan 1, 1601 UTC.
        let ticks_100ns = ((ft.high as u64) << 32) | ft.low as u64;
        ticks_100ns as u128 * 100u128
    }

    pub fn open_readonly(path: &Path) -> std::io::Result<Handle> {
        let wide: Vec<u16> = path
            .as_os_str()
            .encode_wide()
            .chain(iter::once(0))
            .collect();

        let handle = unsafe {
            CreateFileW(
                wide.as_ptr(),
                GENERIC_READ,
                FILE_SHARE_READ,
                std::ptr::null(),
                OPEN_EXISTING,
                FILE_ATTRIBUTE_NORMAL,
                std::ptr::null_mut(),
            )
        };

        if handle == INVALID_HANDLE_VALUE {
            return Err(std::io::Error::last_os_error());
        }

        Ok(handle)
    }

    pub fn close_handle(handle: Handle) {
        unsafe {
            let _ = CloseHandle(handle);
        }
    }

    pub fn process_id() -> u32 {
        unsafe { GetCurrentProcessId() }
    }
}

#[cfg(unix)]
mod platform {
    use super::{RuntimeArch, WoaCapabilities};
    use std::ffi::c_int;
    use std::os::fd::{FromRawFd, IntoRawFd};
    use std::path::Path;

    #[repr(C)]
    struct Timespec {
        tv_sec: i64,
        tv_nsec: i64,
    }

    unsafe extern "C" {
        fn clock_gettime(clk_id: c_int, tp: *mut Timespec) -> c_int;
        fn getpid() -> c_int;
    }

    const CLOCK_REALTIME: c_int = 0;
    const CLOCK_MONOTONIC: c_int = 1;

    fn compile_target_arch() -> RuntimeArch {
        if cfg!(target_arch = "x86") {
            RuntimeArch::X86
        } else if cfg!(target_arch = "x86_64") {
            RuntimeArch::X64
        } else if cfg!(target_arch = "aarch64") {
            RuntimeArch::Arm64
        } else {
            RuntimeArch::Unknown
        }
    }

    pub type Handle = c_int;

    pub fn woa_capabilities() -> WoaCapabilities {
        let arch = compile_target_arch();
        WoaCapabilities {
            host_arch: arch,
            process_arch: arch,
            x64_emulation_available: false,
            x86_emulation_available: false,
        }
    }

    fn read_clock(clock: c_int) -> u128 {
        let mut ts = Timespec {
            tv_sec: 0,
            tv_nsec: 0,
        };

        let rc = unsafe { clock_gettime(clock, &mut ts as *mut Timespec) };
        if rc != 0 || ts.tv_sec < 0 || ts.tv_nsec < 0 {
            return 0;
        }

        (ts.tv_sec as u128 * 1_000_000_000u128) + ts.tv_nsec as u128
    }

    pub fn monotonic_time_ns() -> u128 {
        read_clock(CLOCK_MONOTONIC)
    }

    pub fn realtime_ns() -> u128 {
        read_clock(CLOCK_REALTIME)
    }

    pub fn open_readonly(path: &Path) -> std::io::Result<Handle> {
        let file = std::fs::OpenOptions::new().read(true).open(path)?;
        Ok(file.into_raw_fd())
    }

    pub fn close_handle(handle: Handle) {
        let _ = unsafe { std::fs::File::from_raw_fd(handle) };
    }

    pub fn process_id() -> u32 {
        unsafe { getpid() as u32 }
    }
}

pub type OsHandle = platform::Handle;

pub fn detect_capabilities() -> WoaCapabilities {
    platform::woa_capabilities()
}

pub fn monotonic_time_ns() -> u128 {
    platform::monotonic_time_ns()
}

pub fn realtime_time_ns() -> u128 {
    platform::realtime_ns()
}

pub fn open_path_readonly(path: impl AsRef<Path>) -> std::io::Result<OsHandle> {
    platform::open_readonly(path.as_ref())
}

pub fn close_path_handle(handle: OsHandle) {
    platform::close_handle(handle)
}

pub fn process_id() -> u32 {
    platform::process_id()
}

#[cfg(feature = "ffi")]
mod ffi {
    use super::{detect_capabilities, monotonic_time_ns, process_id, realtime_time_ns, RuntimeArch};

    #[repr(C)]
    pub struct W2nixCapabilities {
        pub host_arch: u32,
        pub process_arch: u32,
        pub x64_emulation_available: bool,
        pub x86_emulation_available: bool,
    }

    fn arch_code(arch: RuntimeArch) -> u32 {
        match arch {
            RuntimeArch::Unknown => 0,
            RuntimeArch::X86 => 1,
            RuntimeArch::X64 => 2,
            RuntimeArch::Arm64 => 3,
        }
    }

    #[no_mangle]
    pub extern "C" fn w2nix_detect_capabilities() -> W2nixCapabilities {
        let caps = detect_capabilities();
        W2nixCapabilities {
            host_arch: arch_code(caps.host_arch),
            process_arch: arch_code(caps.process_arch),
            x64_emulation_available: caps.x64_emulation_available,
            x86_emulation_available: caps.x86_emulation_available,
        }
    }

    #[no_mangle]
    pub extern "C" fn w2nix_monotonic_time_ns() -> u128 {
        monotonic_time_ns()
    }

    #[no_mangle]
    pub extern "C" fn w2nix_realtime_time_ns() -> u128 {
        realtime_time_ns()
    }

    #[no_mangle]
    pub extern "C" fn w2nix_process_id() -> u32 {
        process_id()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn capabilities_have_arch_values() {
        let caps = detect_capabilities();
        assert!(matches!(
            caps.host_arch,
            RuntimeArch::Unknown | RuntimeArch::X86 | RuntimeArch::X64 | RuntimeArch::Arm64
        ));
        assert!(matches!(
            caps.process_arch,
            RuntimeArch::Unknown | RuntimeArch::X86 | RuntimeArch::X64 | RuntimeArch::Arm64
        ));
    }

    #[test]
    fn monotonic_clock_is_nonzero() {
        assert!(monotonic_time_ns() > 0);
    }

    #[test]
    fn process_id_is_nonzero() {
        assert!(process_id() > 0);
    }
}
