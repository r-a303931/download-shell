use std::ffi::{c_char, c_void};

#[allow(non_camel_case_types)]
type iptc_handle_t = *const c_void;

unsafe extern "C" {
    fn iptc_init(name: *const c_char) -> iptc_handle_t;
}
