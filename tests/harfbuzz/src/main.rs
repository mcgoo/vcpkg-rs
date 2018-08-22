use std::ffi::CStr;
use std::str;

extern "C" {
    pub fn hb_version_string() -> *const ::std::os::raw::c_char;
}

fn main() {
    unsafe {
        println!(
            "HarfBuzz version {:?}",
            str::from_utf8(CStr::from_ptr(hb_version_string()).to_bytes())
        );
    }
}
