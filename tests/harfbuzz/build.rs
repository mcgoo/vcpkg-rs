extern crate vcpkg;

fn main() {
    vcpkg::find_package("harfbuzz").unwrap();
}
