//! A build dependency for Cargo libraries to find libraries in a
//! [vcpkg](https://github.com/Microsoft/vcpkg) tree.
//!
//! A number of environment variables are available to globally configure which
//! libraries are selected.
//!
//! * `VCPKG_ROOT` - Set the directory to look in for a vcpkg installation. If
//! it is not set, vcpkg will use the user-wide installation if one has been
//! set up with `vcpkg integrate install`
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
//! ```Batchfile
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
    required_libs: Vec<LibNames>, // copy_to_target: bool,
}

#[derive(Debug)]
pub struct Library {
    pub link_paths: Vec<PathBuf>,
    pub include_paths: Vec<PathBuf>,
    pub cargo_metadata: Vec<String>,

    /// libraries found are static
    pub is_static: bool,

    // DLLs found
    pub found_dlls: Vec<PathBuf>,

    // static libs or import libs found
    pub found_libs: Vec<PathBuf>,
}

enum MSVCTarget {
    X86,
    X64,
}

impl fmt::Display for MSVCTarget {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            MSVCTarget::X86 => write!(f, "x86-windows"),
            MSVCTarget::X64 => write!(f, "x64-windows"),
        }
    }
}

#[derive(Debug)] // need Display?
pub enum Error {
    /// Aborted because of `*_NO_VCPKG` environment variable.
    ///
    /// Contains the name of the responsible environment variable.
    EnvNoPkgConfig(String),

    /// Only MSVC ABI is supported
    NotMSVC,

    /// Can't find a vcpkg tree
    VcpkgNotFound(String),

    /// Library not found in vcpkg tree
    LibNotFound(String),

    #[doc(hidden)]
    __Nonexhaustive,
}

impl error::Error for Error {
    fn description(&self) -> &str {
        match *self {
            Error::EnvNoPkgConfig(_) => "vcpkg requested to be aborted",
            Error::NotMSVC => "vcpkg only can only find libraries for MSVC ABI 64 bit builds",
            Error::VcpkgNotFound(_) => "could not find vcpkg tree",
            Error::LibNotFound(_) => "could not find library in vcpkg tree",
            // Error::LibNotFound(_) => "could not find library in vcpkg tree",
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
                       "this vcpkg build helper can only find libraries built for the MSVC ABI.")
            } 
            Error::VcpkgNotFound(ref detail) => write!(f, "Could not find vcpkg tree: {}", detail),
            Error::LibNotFound(ref detail) => {
                write!(f, "Could not find library in vcpkg tree {}", detail)
            }
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
    let local_app_data = try!(env::var("LOCALAPPDATA").map_err(|_| {
        Error::VcpkgNotFound("Failed to read LOCALAPPDATA environment variable".to_string())
    })); // not present or can't utf8
    let vcpkg_user_targets_path =
        Path::new(local_app_data.as_str()).join("vcpkg").join("vcpkg.user.targets");

    let file = try!(File::open(vcpkg_user_targets_path.clone()).map_err(|_| {
        Error::VcpkgNotFound("No vcpkg.user.targets found. run 'vcpkg integrate install' or set \
                              VCPKG_ROOT environment variable."
            .to_string())
    }));
    let file = BufReader::new(&file);

    for line in file.lines() {
        let line = try!(line.map_err(|_| {
            Error::VcpkgNotFound(format!("Parsing of {} failed.",
                                         vcpkg_user_targets_path.to_string_lossy().to_owned()))
        }));
        let mut split = line.split("Project=\"");
        split.next(); // eat anything before Project="
        if let Some(found) = split.next() {
            // " is illegal in a Windows pathname
            if let Some(found) = found.split_terminator("\"").next() {
                let mut vcpkg_root = PathBuf::from(found);
                if !(vcpkg_root.pop() && vcpkg_root.pop() && vcpkg_root.pop() && vcpkg_root.pop()) {
                    return Err(Error::VcpkgNotFound(format!("Could not find vcpkg root above {}",
                                                            found)));
                }
                return Ok(vcpkg_root);
            }
        }
    }

    Err(Error::VcpkgNotFound(format!("Project location not found parsing {}.",
                                     vcpkg_user_targets_path.to_string_lossy().to_owned())))
}

fn validate_vcpkg_root(path: &PathBuf) -> Result<(), Error> {

    let mut vcpkg_root_path = path.clone();
    vcpkg_root_path.push(".vcpkg-root");

    if vcpkg_root_path.exists() {
        Ok(())
    } else {
        Err(Error::VcpkgNotFound(format!("Could not find vcpkg root at {}",
                                         vcpkg_root_path.to_string_lossy())))
    }
}

/// names of the libraries
struct LibNames {
    lib_stem: String,
    dll_stem: String,
}

impl Config {
    pub fn new() -> Config {
        Config {
            // override environment selection of static or dll
            statik: None,

            // should the cargo metadata actually be emitted
            cargo_metadata: true,
/*
            // should include lines be included in the cargo metadata
            
            //include_includes: false,
*/
            required_libs: Vec::new(),

           // copy_to_target: false,
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

    /// Override the name of the library to look for if it differs from the package name.
    ///
    /// This may be called more than once if multiple libs are required.
    /// All libs must be found for the probe to succeed. `.probe()` must
    /// be run with a different configuration to look for libraries under one of several names.
    /// `.libname("ssleay32")` will look for ssleay32.lib and also ssleay32.dll if
    /// dynamic linking is selected.
    pub fn lib_name(&mut self, lib_stem: &str) -> &mut Config {
        self.required_libs.push(LibNames {
            lib_stem: lib_stem.to_owned(),
            dll_stem: lib_stem.to_owned(),
        });
        self
    }

    /// Override the name of the library to look for if it differs from the package name.
    ///
    /// This may be called more than once if multiple libs are required.
    /// All libs must be found for the probe to succeed. `.probe()` must
    /// be run with a different configuration to look for libraries under one of several names.
    /// `.lib_names("libcurl_imp","curl")` will look for libcurl_imp.lib and also curl.dll if
    /// dynamic linking is selected.
    pub fn lib_names(&mut self, lib_stem: &str, dll_stem: &str) -> &mut Config {
        self.required_libs.push(LibNames {
            lib_stem: lib_stem.to_owned(),
            dll_stem: dll_stem.to_owned(),
        });
        self
    }

    /// Define whether metadata should be emitted for cargo allowing it to
    /// automatically link the binary. Defaults to `true`.
    pub fn cargo_metadata(&mut self, cargo_metadata: bool) -> &mut Config {
        self.cargo_metadata = cargo_metadata;
        self
    }

    /// Find the library `port_name` in a vcpkg tree.
    ///
    /// This will use all configuration previously set to select the
    /// architecture and linkage.
    pub fn probe(&mut self, port_name: &str) -> Result<Library, Error> {

        // if no overrides have been selected, then the vcpkg port name
        // is the the .lib name and the .dll name
        if self.required_libs.is_empty() {
            self.required_libs.push(LibNames {
                lib_stem: port_name.to_owned(),
                dll_stem: port_name.to_owned(),
            });
        }

        let abort_var_name = format!("{}_NO_VCPKG", envify(port_name));
        if env::var_os(&abort_var_name).is_some() {
            return Err(Error::EnvNoPkgConfig(abort_var_name));
        }

        let msvc_arch = try!(msvc_target());

        let vcpkg_root = try!(find_vcpkg_root());
        try!(validate_vcpkg_root(&vcpkg_root));

        let static_lib = self.is_static(port_name);

        let mut lib = Library::new(static_lib);

        let mut base = vcpkg_root;
        base.push("installed");
        let static_appendage = if static_lib {
            "-static"
        } else {
            ""
        };

        let vcpkg_triple = format!("{}{}", msvc_arch.to_string(), static_appendage);
        base.push(vcpkg_triple);

        let lib_path = base.join("lib");
        let bin_path = base.join("bin");
        let include_path = base.join("include");
        lib.cargo_metadata
            .push(format!("cargo:rustc-link-search=native={}",
                          lib_path.to_str().expect("failed to convert string type")));
        if !static_lib {
            lib.cargo_metadata
                .push(format!("cargo:rustc-link-search=native={}",
                              bin_path.to_str().expect("failed to convert string type")));
        }
        lib.include_paths.push(include_path);
        lib.link_paths.push(lib_path.clone());
        drop(port_name);
        for required_lib in &self.required_libs {
            if static_lib {
                lib.cargo_metadata
                    .push(format!("cargo:rustc-link-lib=static={}", required_lib.lib_stem));
            } else {
                lib.cargo_metadata.push(format!("cargo:rustc-link-lib={}", required_lib.lib_stem));
            }

            // verify that the library exists
            let mut lib_location = PathBuf::from(lib_path.clone());
            lib_location.push(required_lib.lib_stem.clone());
            lib_location.set_extension("lib");

            if !lib_location.exists() {
                return Err(Error::LibNotFound(lib_location.display().to_string()));
            }
            lib.found_libs.push(lib_location);

            // verify that the DLL exists
            if !static_lib {
                let mut lib_location = PathBuf::from(bin_path.clone());
                lib_location.push(required_lib.dll_stem.clone());
                lib_location.set_extension("dll");

                if !lib_location.exists() {
                    return Err(Error::LibNotFound(lib_location.display().to_string()));
                }
                lib.found_dlls.push(lib_location);
            }
        }

        // if self.copy_to_target {
        //     if let Some(target_dir) = env::var_os("OUT_DIR") {
        //         for file in &lib.found_dlls {
        //             let mut dest_path = Path::new(target_dir.as_os_str()).to_path_buf();
        //             dest_path.push(Path::new(file.file_name().unwrap()));
        //             fs::copy(file, &dest_path)
        //                 .map_err(|_| {
        //                     Error::LibNotFound(format!("Can't copy file {} to {}",
        //                                                file.to_string_lossy(),
        //                                                dest_path.to_string_lossy()))
        //                 })?;

        //             println!("warning: copied {} to {}",
        //                      file.to_string_lossy(),
        //                      dest_path.to_string_lossy());
        //         }
        //     } else {
        //         return Err(Error::LibNotFound("Can't copy file".to_owned())); // TODO:
        //     }
        // }

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
    pub fn new(is_static: bool) -> Library {
        Library {
            include_paths: Vec::new(),
            link_paths: Vec::new(),
            cargo_metadata: Vec::new(),
            is_static: is_static,
            found_dlls: Vec::new(),
            found_libs: Vec::new(),
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

fn msvc_target() -> Result<MSVCTarget, Error> {
    let target = env::var("TARGET").unwrap_or(String::new());
    if !target.contains("-pc-windows-msvc") {
        Err(Error::NotMSVC)
    } else if target.starts_with("x86_64-") {
        Ok(MSVCTarget::X64)
    } else {
        // everything else is x86
        Ok(MSVCTarget::X86)
    }
}