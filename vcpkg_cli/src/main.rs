extern crate clap;
extern crate vcpkg;

use clap::{App, AppSettings, Arg, SubCommand};
use std::env;

fn main() {
    let app = App::new("vcpkg library finder")
        .about("Allows examining what vcpkg will find in a build script")
        .setting(AppSettings::SubcommandRequired)
        .arg(
            Arg::with_name("target")
                .short("t")
                .long("target")
                .value_name("TARGET TRIPLE")
                .help("the rust toolchain triple to find libraries for")
                .takes_value(true)
                .default_value("x86_64-pc-windows-msvc"),
        ).subcommand(
            SubCommand::with_name("probe")
                .about("try to find a package")
                .arg(
                    Arg::with_name("package")
                        .index(1)
                        .required(true)
                        .help("probe for a library and display paths and cargo metadata"),
                ).arg(
                    Arg::with_name("linkage")
                        .short("l")
                        .long("linkage")
                        .takes_value(true)
                        .possible_values(&["dll", "static"]),
                ),
        );

    let matches = app.get_matches();

    // set TARGET as if we are running under cargo
    env::set_var("TARGET", matches.value_of("target").unwrap());

    if let Some(matches) = matches.subcommand_matches("probe") {
        let lib_name = matches.value_of("package").unwrap();

        let mut cfg = vcpkg::Config::new();
        cfg.cargo_metadata(false);
        cfg.copy_dlls(false);
        if let Some(linkage) = matches.value_of("linkage") {
            match &linkage {
                &"dll" => {
                    // do nothing
                }
                &"static" => {
                    env::set_var("CARGO_CFG_TARGET_FEATURE", "crt-static");
                }
                _ => unreachable!(),
            }
        }

        match cfg.find_package(lib_name) {
            Ok(lib) => {
                println!("Found library {}", lib_name);

                if !lib.include_paths.is_empty() {
                    println!("Include paths:");
                    for line in &lib.include_paths {
                        println!("  {}", line.as_os_str().to_str().unwrap());
                    }
                }

                if !lib.link_paths.is_empty() {
                    println!("Library paths:");
                    for line in &lib.link_paths {
                        println!("  {}", line.as_os_str().to_str().unwrap());
                    }
                }

                if !lib.link_paths.is_empty() {
                    println!("Runtime Library paths:");
                    for line in &lib.dll_paths {
                        println!("  {}", line.as_os_str().to_str().unwrap());
                    }
                }

                if !lib.cargo_metadata.is_empty() {
                    println!("Cargo metadata:");
                    for line in &lib.cargo_metadata {
                        println!("  {}", line);
                    }
                }
                if !lib.found_dlls.is_empty() {
                    println!("Found DLLs:");
                    for line in &lib.found_dlls {
                        println!("  {}", line.display());
                    }
                }
                if !lib.found_libs.is_empty() {
                    println!("Found libs:");
                    for line in &lib.found_libs {
                        println!("  {}", line.display());
                    }
                }
            }
            Err(err) => {
                println!("Failed:  {}", err);
            }
        }
    }
}
