fn main() {
    println!(
        "RUSTFLAGS={}",
        std::env::var("RUSTFLAGS").unwrap_or_default()
    );
    // panic!("CARGO={}", std::env::var("CARGO").unwrap());
    vcpkg::find_package("sdl2").unwrap();
}
