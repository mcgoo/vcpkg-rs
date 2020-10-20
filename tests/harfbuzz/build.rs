extern crate cc;
extern crate vcpkg;

fn main() {
    let libs = vcpkg::Config::new().find_package("harfbuzz").unwrap();

    // vcpkg-rs is not capable of working out the correct order to link
    // libraries in. This only matters on Linux at present. (vcpkg itself
    // does fine, but vcpkg-rs needs to work out how to get the link order
    // from the it.)
    println!("cargo:rustc-link-lib=brotlicommon-static");

    let mut build = cc::Build::new();
    build.file("src/test.c");
    for inc in libs.include_paths {
        build.include(&inc);
        println!("inc={}", inc.to_string_lossy());
    }
    build.compile("harfbuzz_header");
}
