extern crate cc;
extern crate vcpkg;

fn main() {
    let libs = vcpkg::Config::new().find_package("harfbuzz").unwrap();

    let mut build = cc::Build::new();
    build.file("src/test.c");
    for inc in libs.include_paths {
        build.include(&inc);
        println!("inc={}", inc.to_string_lossy());
    }
    build.compile("harfbuzz_header");
}
