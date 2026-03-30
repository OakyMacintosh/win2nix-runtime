use core::ffi::{c_char, c_int, c_void};

const O_RDONLY: c_int = 0;
const O_TEXT: c_int = 0x4000;
const CW_PERFILE: c_int = 44;

#[repr(C)]
pub struct PerProcess {
    _private: [u8; 0],
}

#[repr(C)]
struct CygwinPerfile {
    name: *const c_char,
    flags: c_int,
}

unsafe extern "C" {
    fn cygwin_internal(command: c_int, ...) -> *mut c_void;
}

#[no_mangle]
pub extern "C" fn cygwin_premain0(
    _argc: c_int,
    _argv: *mut *mut c_char,
    _myself: *mut PerProcess,
) {
    static EMPTY: [u8; 1] = [0];
    let mut table = [
        CygwinPerfile {
            name: EMPTY.as_ptr() as *const c_char,
            flags: O_RDONLY | O_TEXT,
        },
        CygwinPerfile {
            name: core::ptr::null(),
            flags: 0,
        },
    ];

    unsafe {
        let _ = cygwin_internal(CW_PERFILE, table.as_mut_ptr());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn per_process_is_opaque() {
        assert_eq!(core::mem::size_of::<PerProcess>(), 0);
    }
}
