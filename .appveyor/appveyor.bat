echo on
SetLocal EnableDelayedExpansion

REM This is the recommended way to choose the toolchain version, according to
REM Appveyor's documentation.
SET PATH=C:\Program Files (x86)\MSBuild\%TOOLCHAIN_VERSION%\Bin;%PATH%

set VCVARSALL="C:\Program Files (x86)\Microsoft Visual Studio %TOOLCHAIN_VERSION%\VC\vcvarsall.bat"
set MSVCYEAR=vs2015
set MSVCVERSION=v140

if [%Platform%] NEQ [x64] goto win32
set TARGET_ARCH=x86_64
set TARGET_PROGRAM_FILES=%ProgramFiles%
rem call %VCVARSALL% amd64
rem if %ERRORLEVEL% NEQ 0 exit 1
goto download

:win32
echo on
if [%Platform%] NEQ [Win32] exit 1
set TARGET_ARCH=i686
set TARGET_PROGRAM_FILES=%ProgramFiles(x86)%
rem call %VCVARSALL% amd64_x86
rem if %ERRORLEVEL% NEQ 0 exit 1
goto download

:download
REM vcvarsall turns echo off
echo on

cd %ORIGINAL_PATH%


git clone https://github.com/Microsoft/vcpkg packages
cd packages
powershell -exec bypass scripts\bootstrap.ps1
vcpkg --triplet=x64-windows install sqlite3 libpq libmysql curl zeromq 
vcpkg integrate install

cd ..
echo on

set RUST_URL=https://static.rust-lang.org/dist/rust-%RUST%-%TARGET_ARCH%-pc-windows-msvc.msi
echo Downloading %RUST_URL%...
mkdir build
powershell -Command "(New-Object Net.WebClient).DownloadFile('%RUST_URL%', 'build\rust-%RUST%-%TARGET_ARCH%-pc-windows-msvc.msi')"
if %ERRORLEVEL% NEQ 0 (
  echo ...downloading Rust failed.
  exit 1
)

start /wait msiexec /i build\rust-%RUST%-%TARGET_ARCH%-pc-windows-msvc.msi INSTALLDIR="%TARGET_PROGRAM_FILES%\Rust %RUST%" /quiet /qn /norestart
if %ERRORLEVEL% NEQ 0 exit 1

set PATH="%TARGET_PROGRAM_FILES%\Rust %RUST%\bin";%PATH%

if [%Configuration%] == [Release] set CARGO_MODE=--release

link /?
cl /?
rustc --version
cargo --version

set RUST_BACKTRACE=1

cargo build --all

cargo test --all

REM cargo build --manifest-path vcpkg\Cargo.toml
REM cargo build --manifest-path vcpkg_cli\Cargo.toml

REM cargo test --manifest-path vcpkg\Cargo.toml
REM cargo test --manifest-path vcpkg\Cargo.toml



cargo run --manifest-path vcpkg_cli\Cargo.toml -- probe sqlite3

cargo run --manifest-path systest\Cargo.toml 
