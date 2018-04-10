@echo off

set VCPKG_PANIC=on
set VCPKG_ROOT=c:\Users\jim\src\vcpkg

rem for %%i in (x64-windows x64-windows-static x86-windows x86-windows-static) DO (
rem     %VCPKG_ROOT%\vcpkg install --triplet=%i% openssl curl
rem )

rem x86_64-pc-windows-gnu i686-pc-windows-gnu
for %%i in (x86_64-pc-windows-msvc i686-pc-windows-msvc) DO (
    set RUSTFLAGS=-Ctarget-feature=+crt-static
    set VCPKGRS_DYNAMIC=
    echo %%i static
    cargo run --target %%i
    cargo clean
    set RUSTFLAGS=
    set VCPKGRS_DYNAMIC=1
    echo %%i dynamic
    cargo run --target %%i
    cargo clean
) 

echo linux
bash -l -c "cargo run"
