@echo off

set VCPKG_PANIC=on
set VCPKG_ROOT=c:\Users\jim\src\vcpkg

rem  
for %%i in (x86_64-pc-windows-msvc i686-pc-windows-msvc x86_64-pc-windows-gnu i686-pc-windows-gnu) DO (
    set VCPKG_ALL_STATIC=on
    set VCPKG_ALL_DYNAMIC=
    echo %%i static
    cargo run --target %%i
    cargo clean
    set VCPKG_ALL_STATIC=
    set VCPKG_ALL_DYNAMIC=on
    echo %%i dynamic
    cargo run --target %%i
    cargo clean
) 

echo linux
bash -l -c "cargo run"
