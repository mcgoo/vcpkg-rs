extern crate vcpkg;
extern crate clap;

use clap::{App, AppSettings, Arg, SubCommand};
use std::env;

fn main() {

    let app = App::new("vcpkg library finder")
        .about("Allows examining what vcpkg will find in a build script")
        .setting(AppSettings::SubcommandRequired)
        .arg(Arg::with_name("target")
            .short("t")
            .long("target")
            .value_name("TARGET TRIPLE")
            .help("the rust toolchain triple to find libraries for")
            .takes_value(true)
            .default_value("x86_64-pc-windows-msvc"))
        .subcommand(SubCommand::with_name("probe")
            .about("try to find a library")
            .arg(Arg::with_name("library")
                .index(1)
                .required(true)
                .help("probe for a library and display paths and cargo metadata"))
            .arg(Arg::with_name("linkage")
                .short("l")
                .long("linkage")
                .takes_value(true)
                .possible_values(&["dll", "static"])));

    let matches = app.get_matches();

    // set TARGET as if we are running under cargo
    env::set_var("TARGET", matches.value_of("target").unwrap());

    if let Some(matches) = matches.subcommand_matches("probe") {

        let lib_name = matches.value_of("library").unwrap();

        let mut cfg = vcpkg::Config::new();
        cfg.cargo_metadata(false);

        if let Some(linkage) = matches.value_of("linkage") {
            match &linkage {
                &"dll" => {
                    cfg.statik(false);
                }
                &"static" => {
                    cfg.statik(true);
                }
                _ => unreachable!(),
            }
        }

        match cfg.probe(lib_name) {
            Ok(lib) => {
                println!("Found library {}", lib_name);

                if !lib.include_paths.is_empty() {
                    println!("Include paths:");
                    for line in &lib.include_paths {
                        println!("\t{}", line.as_os_str().to_str().unwrap());
                    }
                }

                if !lib.link_paths.is_empty() {
                    println!("Library paths:");
                    for line in &lib.link_paths {
                        println!("\t{}", line.as_os_str().to_str().unwrap());
                    }
                }

                if !lib.cargo_metadata.is_empty() {
                    println!("Cargo metadata:");
                    for line in &lib.cargo_metadata {
                        println!("\t{}", line);
                    }
                }
            }
            Err(err) => {
                println!("Failed:  {}", err);

            }
        }
    }
}