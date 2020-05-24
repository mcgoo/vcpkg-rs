fn main() {
    // panic!("CARGO={}", std::env::var("CARGO").unwrap());
    vcpkg::find_package("sdl2").unwrap();
}
