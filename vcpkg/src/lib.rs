//! A build dependency for Cargo libraries to find libraries in a `vcpkg` tree.
//!
//! A number of environment variables are available to globally configure which
//! libraries are selected.
//!
//! * `FOO_NO_VCPKG` - if set, vcpkg will not attempt to find the
//! library named `foo`.
//!
//! There are also a number of environment variables which can configure how a
//! library is linked to (dynamically vs statically). These variables control
//! whether the `--static` flag is passed. Note that this behavior can be
//! overridden by configuring explicitly on `Config`. The variables are checked
//! in the following order:
//!
//! * `FOO_STATIC` - find the static version of `foo`
//! * `FOO_DYNAMIC` - find the dll version of  `foo`
//! * `VCPKG_ALL_STATIC` - find the static version of all libraries
//! * `VCPKG_ALL_DYNAMIC` - find the dll version of all libraries
//!
//! If the search was successful all appropriate Cargo metadata will be printed
//! on stdout.
//!
//! This cargo build helper is derived from and intended to work like the
//! `pkg-config` build helper to the extent that is possible, but `pkg-config`
//! the tool has functionality that `vcpkg` does not. In particular, vcpkg
//! does not allow mapping from a package name to the libs that it provides,
//! so this build helper must be called once for each library that is required
//! rather than once for the overall package. A better interface is no doubt
//! possible.
//!
//! There is a companion crate `vcpkg_cli` that allows testing of environment
//! and flag combinations.
//!
//! ```no_run
//! C:\src> vcpkg_cli probe -l static mysqlclient
//! Found library mysqlclient
//! Include paths:
//!         C:\src\diesel_build\vcpkg-dll\installed\x64-windows-static\include
//! Library paths:
//!         C:\src\diesel_build\vcpkg-dll\installed\x64-windows-static\lib
//! Cargo metadata:
//!         cargo:rustc-link-search=native=C:\src\diesel_build\vcpkg-dll\installed\x64-windows-static\lib
//!         cargo:rustc-link-lib=static=mysqlclient
//! ```
//!
//! there is a test of diesel and it's dependencies that should be easy to set
//! up at https://github.com/mcgoo/vcpkg_diesel_build
//! check out that project and then
//! ```no_run
//! make_vcpkg_static
//! build_diesel_static
//! ```
//! or
//! ```no_run
//! make_vcpkg_dll
//! build_diesel_dll
//! ```
//! and you should get a working binary.
//!
//!
//! more notes for this work in progress :-
//!
//! could run vcpkg and parse it's output to determine what package versions are
//! installed. at present it just generates plausible link lines
//!
//! vcpkg has common include and lib dirs so there is a chance that someone is
//! going to end up picking up a vcpkg lib on their link line in preference to
//! some other version at some point. I believe cmake handles this by using
//! absolute paths for libs wherever possible.
//!
//! vcpkg has a per-package output dir that looks like it would be helpful,
//! but at present it is undocumented and subject to change. (what I read
//! mentioned the possibility of compressing the contents.)
//!
//! x86 is not done, only x86_64
//!
//! could automatically copy dlls to the target directory so that dynamic
//! builds will actually run
//!
//! there is a lib\no_auto_link folder that some packages generate that needs
//! to be added to the link line
//!
//! should it link rust debug builds against debug libraries? it does not at
//! present.
//!
//! there is some (expected) weirdness when you link to a static lib that needs
//! other components to link against. I was only able to resolve libpq.lib
//! needing the openssl libs (ssleay32.lib and libssl32.lib) by making the
//! top level binary crate create a reference using `extern crate openssl-sys`.

use std::ascii::AsciiExt;
use std::env;
use std::error;
use std::fs::File;
use std::fmt;
use std::io::{BufRead, BufReader};
use std::path::{PathBuf, Path};

// #[derive(Clone)]
pub struct Config {
    statik: Option<bool>,
    cargo_metadata: bool,
}

#[derive(Debug)]
pub struct Library {
    pub link_paths: Vec<PathBuf>,
    pub include_paths: Vec<PathBuf>,
    pub cargo_metadata: Vec<String>,
}

#[derive(Debug)] // need Display?
pub enum Error {
    /// Aborted because of `*_NO_VCPKG` environment variable.
    ///
    /// Contains the name of the responsible environment variable.
    EnvNoPkgConfig(String),

    /// Only MSVC ABI is supported
    NotMSVC,

    // VcpkgNotFound()
    #[doc(hidden)]
    __Nonexhaustive,
}

impl error::Error for Error {
    fn description(&self) -> &str {
        match *self {
            Error::EnvNoPkgConfig(_) => "vcpkg requested to be aborted",
            Error::NotMSVC => "vcpkg only can only find libraries for MSVC ABI 64 bit builds",
            // Error::Command { .. } => "failed to run vcpkg",
            // Error::Failure { .. } => "vcpkg did not exit sucessfully",
            Error::__Nonexhaustive => panic!(),
        }
    }

    fn cause(&self) -> Option<&error::Error> {
        match *self {
            // Error::Command { ref cause, .. } => Some(cause),
            _ => None,
        }
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        match *self {
            Error::EnvNoPkgConfig(ref name) => write!(f, "Aborted because {} is set", name),
            Error::NotMSVC => {
                write!(f,
                       "vcpkg can only find libraries built for the 64 bit MSVC ABI.")
            }
            // Error::Command { ref command, ref cause } => {
            //     write!(f, "Failed to run `{}`: {}", command, cause)
            // }
            // Error::Failure { ref command, ref output } => {
            //     let stdout = str::from_utf8(&output.stdout).unwrap();
            //     let stderr = str::from_utf8(&output.stderr).unwrap();
            //     try!(write!(f, "`{}` did not exit successfully: {}", command, output.status));
            //     if !stdout.is_empty() {
            //         try!(write!(f, "\n--- stdout\n{}", stdout));
            //     }
            //     if !stderr.is_empty() {
            //         try!(write!(f, "\n--- stderr\n{}", stderr));
            //     }
            //     Ok(())
            // }
            Error::__Nonexhaustive => panic!(),
        }
    }
}

pub fn probe_library(name: &str) -> Result<Library, Error> {
    Config::new().probe(name)
}

fn find_vcpkg_root() -> Result<PathBuf, Error> {

    // prefer the setting from the environment is there is one
    if let Some(path) = env::var_os("VCPKG_ROOT") {
        return Ok(PathBuf::from(path));
    }

    // see if there is a per-user vcpkg tree that has been integrated into msbuild
    // using `vcpkg integrate install`
    let local_app_data = env::var("LOCALAPPDATA").map_err(|_| Error::NotMSVC)?; // not present or can't utf8
    let vcpkg_user_targets_path =
        Path::new(local_app_data.as_str()).join("vcpkg").join("vcpkg.user.targets");

    let file = File::open(vcpkg_user_targets_path).map_err(|_| Error::NotMSVC)?; // TODO:
    let file = BufReader::new(&file);

    for line in file.lines() {
        let line = line.map_err(|_| Error::NotMSVC)?;
        let mut split = line.split("Project=\"");
        split.next(); // eat anything before Project="
        if let Some(found) = split.next() {
            // " is illegal in a Windows pathname
            if let Some(found) = found.split_terminator("\"").next() {
                return Ok(PathBuf::from(found));
            }
        }
    }

    Err(Error::NotMSVC)
}

fn validate_vcpkg_root(path: &PathBuf) -> Result<(), Error> {

    let mut vcpkg_root_path = path.clone();
    vcpkg_root_path.push(".vcpkg-root");

    if vcpkg_root_path.exists() {
        Ok(())
    } else {
        Err(Error::NotMSVC)
    }
}

impl Config {
    pub fn new() -> Config {
        Config {
            statik: None,
            cargo_metadata: true,
        }
    }

    /// Indicate whether to look for a static lib.
    ///
    /// This will override the inference from environment variables described in
    /// the crate documentation.
    pub fn statik(&mut self, statik: bool) -> &mut Config {
        self.statik = Some(statik);
        self
    }

    /// Define whether metadata should be emitted for cargo allowing it to
    /// automatically link the binary. Defaults to `true`.
    pub fn cargo_metadata(&mut self, cargo_metadata: bool) -> &mut Config {
        self.cargo_metadata = cargo_metadata;
        self
    }

    /// Find the library `name` in a vcpkg tree.
    ///
    /// This will use all configuration previously set to select the
    /// architecture and linkage.
    pub fn probe(&self, name: &str) -> Result<Library, Error> {

        let abort_var_name = format!("{}_NO_VCPKG", envify(name));
        if env::var_os(&abort_var_name).is_some() {
            return Err(Error::EnvNoPkgConfig(abort_var_name));
        } else if !target_supported() {
            return Err(Error::NotMSVC);
        }

        // for the moment bail out if it's not an x86_64-pc-windows-msvc build
        let target = env::var("TARGET").unwrap_or(String::new());
        if !target.contains("x86_64-pc-windows-msvc") {
            return Err(Error::NotMSVC);
        }

        let vcpkg_root = find_vcpkg_root()?;
        validate_vcpkg_root(&vcpkg_root)?;

        let mut lib = Library::new();

        let static_lib = self.is_static(name);

        let mut base = vcpkg_root;
        base.push("installed");
        if static_lib {
            base.push("x64-windows-static");
        } else {
            base.push("x64-windows");
        }
        let base_str = base.to_str().expect("failed to convert string type");

        lib.cargo_metadata.push(format!("cargo:rustc-link-search=native={}\\lib", base_str));

        if static_lib {
            lib.cargo_metadata.push(format!("cargo:rustc-link-lib=static={}", name));
        } else {
            lib.cargo_metadata.push(format!("cargo:rustc-link-lib={}", name));
        }

        let val = format!("{}\\include", base_str);
        lib.include_paths.push(PathBuf::from(val));
        let lib_path = format!("{}\\lib", base_str);
        lib.link_paths.push(PathBuf::from(&lib_path));

        // actually verify that the library exists
        let mut lib_location = PathBuf::from(lib_path);
        lib_location.push(name);
        lib_location.set_extension("lib");
        println!("{:?}", lib_location);
        if !lib_location.exists() {

            return Err(Error::NotMSVC);
        }


        if self.cargo_metadata {
            for line in &lib.cargo_metadata {
                println!("{}", line);
            }
        }

        Ok(lib)
    }


    fn is_static(&self, name: &str) -> bool {
        self.statik.unwrap_or_else(|| infer_static(name))
    }
}

impl Library {
    pub fn new() -> Library {
        Library {
            include_paths: Vec::new(),
            link_paths: Vec::new(),
            cargo_metadata: Vec::new(),
        }
    }
}

fn infer_static(name: &str) -> bool {
    let name = envify(name);
    if env::var_os(&format!("{}_STATIC", name)).is_some() {
        true
    } else if env::var_os(&format!("{}_DYNAMIC", name)).is_some() {
        false
    } else if env::var_os("VCPKG_ALL_STATIC").is_some() {
        true
    } else if env::var_os("VCPKG_ALL_DYNAMIC").is_some() {
        false
    } else {
        false
    }
}

fn envify(name: &str) -> String {
    name.chars()
        .map(|c| c.to_ascii_uppercase())
        .map(|c| {
            if c == '-' {
                '_'
            } else {
                c
            }
        })
        .collect()
}

pub fn target_supported() -> bool {
    let target = env::var("TARGET").unwrap_or(String::new());
    target.contains("msvc")
}