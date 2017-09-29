extern crate curl;
extern crate libz_sys;
extern crate openssl_sys;

#[cfg(feature = "extras")]
extern crate zmq_sys;

use std::ffi::CStr;

fn main() {
    println!("curl version is {:?}!", curl::Version::get().version());

    unsafe {
        println!(
            "zlib version is {:?}!",
            CStr::from_ptr(libz_sys::zlibVersion())
        );
    }

    //unsafe{  println!("openssl version is {:?}!", CStr::from_ptr(openssl_sys::SSLEAY_VERSION));}
    unsafe {
        openssl_sys::SSL_library_init();
    }
    //  println!("openssl version is {}!", openssl_sys::OPENSSL_VERSION);

    //  unsafe {let ctx = zmq_sys::zmq_init(1); }

    test_extras();
}

#[cfg(feature = "extras")]
fn test_extras() {
    let mut major = 0;
    let mut minor = 0;
    let mut patch = 0;
    unsafe {
        zmq_sys::zmq_version(&mut major, &mut minor, &mut patch);
    }
    println!("zmq version {}.{}.{}", major, minor, patch);
}

#[cfg(not(feature = "extras"))]
fn test_extras() {}
