use anyhow::{bail, Context};
//use indicatif::{ProgressBar, ProgressStyle};
use serde::Deserialize;
use std::{
    collections::BTreeMap,
    fs::File,
    io::{BufRead, BufReader, Write},
    process::{Command, Output, Stdio},
    str,
    time::SystemTime,
};
use structopt::StructOpt;
use termcolor::{Color, ColorChoice, ColorSpec, StandardStream, WriteColor};
use vcpkg::{find_vcpkg_root, Config};

// settings for a specific Rust target
#[derive(Debug, Deserialize)]
struct Target {
    triplet: Option<String>,
    // this install key for a specific target overrides the main entry
    // so a the target can opt out of installing packages
    // #[serde(default = "Vec::new")]
    install: Option<Vec<String>>,
}

#[derive(Debug, Deserialize)]
struct Vcpkg {
    vcpkg_root: Option<String>,
    #[serde(default = "BTreeMap::new")]
    target: BTreeMap<String, Target>,
    branch: Option<String>,
    rev: Option<String>,
    git: Option<String>,
    tag: Option<String>,
    #[serde(default = "Vec::new")]
    install: Vec<String>,
}
#[derive(Debug, Deserialize)]
struct Metadata {
    vcpkg: Vcpkg,
}
#[derive(Debug, PartialEq, StructOpt)]
/// Install vcpkg and build packages
///
/// This program clones vcpkg from the specified source and
/// compiles it. It then builds packages required by crates
/// that are depended on by the top level crate being built.
#[structopt(rename_all = "kebab-case")]
struct Opt {
    ///
    #[structopt(short, long)]
    verbose: bool,

    // #[structopt(long)]
    // manifest_path: Option<String>,
    #[structopt(subcommand)]
    sub: Subcommands,
}

#[derive(Debug, PartialEq, StructOpt)]
enum Subcommands {
    /// Build packages
    ///
    /// This command will clone or update a vcpkg tree to the version specified
    /// in Cargo.toml and build the required packages.
    Build {
        #[structopt(long)]
        /// Path to Cargo.toml
        manifest_path: Option<String>,

        #[structopt(long)]
        /// Build for the target triple
        target: Option<String>,
    },
    // `external_subcommand` tells structopt to put
    // all the extra arguments into this Vec
    // #[structopt(external_subcommand)]
    // Cargo(Vec<String>),
}

fn main() {
    // cargo passes us the "vcpkg" arg when it calls us. Drop it before
    // parsing the arg list so t doesn't end up the usage description
    let mut args = std::env::args().collect::<Vec<_>>();
    if args.len() >= 2 && args[1] == "vcpkg" {
        args.remove(1);
    }
    let args = Opt::from_iter(args);

    match args.sub {
        Subcommands::Build { .. } => {
            build(args).unwrap_or_else(|e| {
                eprintln!("cargo-vcpkg: {}", e);
                std::process::exit(1);
            });
        }
    }
}
//use std::io::Write;
fn build(opt: Opt) -> Result<(), anyhow::Error> {
    let start_time = SystemTime::now();

    let target_triple = target_triple();

    let verbose = opt.verbose;

    let mut args = std::env::args().skip_while(|val| !val.starts_with("--manifest-path"));
    let mut cmd = cargo_metadata::MetadataCommand::new();
    // opt.manifest_path.map(|p| cmd.manifest_path(p));

    match args.next() {
        Some(p) if p == "--manifest-path" => {
            cmd.manifest_path(args.next().unwrap());
        }
        Some(p) => {
            cmd.manifest_path(p.trim_start_matches("--manifest-path="));
        }
        None => {}
    }
    let metadata = cmd.exec()?;

    let resolve = metadata.resolve.as_ref().unwrap();

    let root_crate = resolve
        .root
        .as_ref()
        .context("cannot run on a virtual manifest, this command requires running against an actual package in this workspace.")?;

    let mut git_url = None;
    let mut vcpkg_ports = Vec::new();
    let mut rev_tag_branch: Option<String> = None;
    let mut vcpkg_triplet = None;
    for p in &metadata.packages {
        if let Ok(v) = serde_json::from_value::<Metadata>(p.metadata.clone()) {
            let v = v.vcpkg;
            let is_root_crate = p.id == *root_crate;

            // only use git url and rev from the root crate
            if v.git.is_some() && is_root_crate {
                git_url = v.git;

                // TODO: check the target and use it's package set if required
                // TODO: get the correct target
                // TODO: make sure to pull if it's a branch
                rev_tag_branch = match (&v.branch, &v.tag, &v.rev) {
                    (Some(b), None, None) => Some(b.into()),
                    (None, Some(t), None) => Some(t.into()),
                    (None, None, Some(r)) => Some(r.into()),
                    _ => {
                        bail!("must specify one of branch,rev,tag for git source");
                    }
                };
            }

            // if there is specific configuration for the target and it has
            // an install key, use that rather than the general install key
            match v.target.get(&target_triple) {
                Some(target) => {
                    if target.install.is_some() {
                        vcpkg_ports.extend_from_slice(&target.install.as_ref().unwrap().as_slice());
                    }
                    if is_root_crate {
                        vcpkg_triplet = target.triplet.clone();
                    }
                }
                _ => {
                    // not found or install is empty
                    vcpkg_ports.extend_from_slice(&v.install.as_slice());
                }
            }
        }
    }

    // should we modify the existing?
    // let mut allow_updates = true;

    // find the vcpkg root
    let vcpkg_root = find_vcpkg_root(&Config::default()).unwrap_or_else(|_| {
        let target_directory = metadata.target_directory.clone();
        let mut vcpkg_root = target_directory.clone();
        vcpkg_root.push("vcpkg");
        vcpkg_root.to_path_buf();
        vcpkg_root
    });
    if verbose {
        println!("vcpkg root is {}", vcpkg_root.display());
    }
    // if it does not exist, clone vcpkg from git
    let mut vcpkg_root_file = vcpkg_root.clone();
    vcpkg_root_file.push(".vcpkg-root");
    if !vcpkg_root_file.exists() {
        let git_url = git_url.context(format!(
            "could not find a vcpkg installation and crate \
        {} does not specify a git repository to clone from.",
            root_crate
        ))?;
        print_tag("Cloning", &git_url);
        let mut cmd = Command::new("git");
        cmd.arg("clone");
        cmd.arg(git_url);
        cmd.arg(&vcpkg_root);
        let _output = run_command(cmd, verbose).context("failed to run git clone")?;

    //eprintln!("git clone done = {:?}", output.status);
    } else {
        print_tag("Fetching", "vcpkg");
        let mut cmd = Command::new("git");
        cmd.arg("fetch");
        cmd.arg("--verbose");
        cmd.arg("--all");
        let output = run_command(cmd, verbose).context("failed to run git fetch")?;

        if !output.status.success() {
            bail!("fetch failed");
        }
    }

    // create a cargo-vcpkg.toml in the vcpkg tree
    let mut cargo_vcpkg_config_file = vcpkg_root.clone();
    cargo_vcpkg_config_file.push("downloads");
    std::fs::create_dir_all(&cargo_vcpkg_config_file)
        .context("could not create downloads directory in vcpkg tree")?;
    cargo_vcpkg_config_file.push("cargo-vcpkg.toml");
    if !cargo_vcpkg_config_file.exists() {
        let mut file =
            File::create(cargo_vcpkg_config_file).context("could not create cargo-vcpkg.toml")?;
        file.write_all(b"# This file was created automatically by cargo-vcpkg\n")?;
    }

    // otherwise, check that the rev is where we want it to be
    // there needs to be some serious thought here because if we are on a branch
    // does this mean we should fetch?

    // check out the required rev
    let rev_tag_branch = rev_tag_branch.unwrap();
    print_tag("Checkout", &format!("rev/tag/branch {}", rev_tag_branch));
    let mut cmd = Command::new("git");
    cmd.arg("checkout");
    cmd.arg(rev_tag_branch);
    cmd.current_dir(&vcpkg_root);
    run_command(cmd, verbose).context("failed to execute process")?;

    // try and run 'vcpkg update' and if it fails or gives the version warning, rebuild it
    let require_bootstrap = match vcpkg_command(&vcpkg_root, &vcpkg_triplet)
        .arg("update")
        .output()
    {
        Ok(output) => {
            if verbose {
                println!("-- stdout --\n{}", String::from_utf8_lossy(&output.stdout));
                println!("-- stderr --\n{}", String::from_utf8_lossy(&output.stderr));
                println!("{:?}", output.status);
            }
            if output.status.success()
                && !str::from_utf8(&output.stdout)
                    .unwrap()
                    .contains("Warning: Different source is available for vcpkg")
            {
                false
            } else {
                true
            }
        }
        Err(_) => true,
    };

    if require_bootstrap {
        print_tag("Compiling", "vcpkg");
        let mut cmd = if cfg!(windows) {
            let mut cmd = Command::new("cmd");
            cmd.arg("/C");
            cmd.arg("bootstrap-vcpkg.bat");
            cmd
        } else {
            let mut cmd = Command::new("sh");
            cmd.arg("-c");
            cmd.arg("./bootstrap-vcpkg.sh");
            cmd
        };
        cmd.arg("-disableMetrics");
        cmd.current_dir(&vcpkg_root);
        run_command(cmd, verbose).context("failed to run vcpkg bootstrap")?;
    }

    // TODO: upgrade anything that is installed
    print_tag("Installing", &vcpkg_ports.join(" "));
    let mut v = vcpkg_command(&vcpkg_root, &vcpkg_triplet);
    v.arg("install");
    v.arg("--recurse");
    v.args(vcpkg_ports.as_slice());
    v.stdout(Stdio::piped());

    let mut output = v.spawn()?;

    let reader = BufReader::new(output.stdout.take().context("could not get stdout")?);

    // let style = ProgressStyle::default_bar()
    //     .progress_chars("=> ")
    //     .template("    Building [{bar:40.cyan/blue}] {pos:>7}/{len:7} {msg}");
    // let bar = ProgressBar::new(10).with_style(style);
    for line in reader.lines().flat_map(Result::ok) {
        parse_build_line(&line).map(|(pkg, triplet, _num, _tot)| {
            print_tag("Compiling", &format!("{} (triplet {})", pkg, triplet))
        });

        if verbose {
            println!("{}", line);
        }
        // bar.set_length(tot);
        // bar.set_position(num);
    }
    // grab anything that is left
    let output = output.wait_with_output()?;

    if !output.status.success() && !verbose {
        println!("-- stdout --\n{}", String::from_utf8_lossy(&output.stdout));
        println!("-- stderr --\n{}", String::from_utf8_lossy(&output.stderr));
        bail!("failed");
    }

    let duration = SystemTime::now().duration_since(start_time).unwrap();
    print_tag("Finished", &format!("in {:0.2}s", duration.as_secs_f32()));
    Ok(())
}

fn target_triple() -> String {
    let mut args = std::env::args().skip_while(|val| !val.starts_with("--target"));
    match args.next() {
        Some(p) if p == "--target" => args.next().unwrap(),
        Some(p) => p.trim_start_matches("--target=").into(),
        None => std::env!("TARGET").into(),
    }
}

fn vcpkg_command(vcpkg_root: &std::path::Path, vcpkg_triplet: &Option<String>) -> Command {
    let mut x = vcpkg_root.to_path_buf();
    if cfg!(windows) {
        x.push("vcpkg.exe");
    } else {
        x.push("vcpkg")
    }
    let mut command = Command::new(x);
    command.current_dir(&vcpkg_root);
    if let Some(triplet) = &vcpkg_triplet {
        command.arg("--triplet");
        command.arg(triplet);
    }
    command
}

fn run_command(mut cmd: Command, verbose: bool) -> Result<Output, anyhow::Error> {
    if verbose {
        cmd.stdout(Stdio::inherit());
        cmd.stderr(Stdio::inherit());
    }
    let output = cmd.output()?;

    if !output.status.success() && !verbose {
        println!("-- stdout --\n{}", String::from_utf8_lossy(&output.stdout));
        println!("-- stderr --\n{}", String::from_utf8_lossy(&output.stderr));
        bail!("failed");
    }

    Ok(output)
}

fn print_tag(tag: &str, detail: &str) {
    let mut stdout = StandardStream::stdout(ColorChoice::Auto);
    stdout
        .set_color(ColorSpec::new().set_fg(Some(Color::Green)).set_bold(true))
        .unwrap();

    print!("{:>12} ", tag);
    stdout.reset().unwrap();
    println!("{}", detail);
}

fn parse_build_line(line: &str) -> Option<(String, String, u64, u64)> {
    let line = Some(line)
        .filter(|line| line.starts_with("Starting package "))
        .map(|line| line.trim_start_matches("Starting package ").to_string())?;

    let progress_and_pkg_trp = line.splitn(2, ":").collect::<Vec<_>>();
    if progress_and_pkg_trp.len() != 2 {
        return None;
    }

    let pkg_with_triplet = progress_and_pkg_trp[1].trim();

    let (pkg, triplet) = match pkg_with_triplet
        .rsplitn(2, ":")
        .collect::<Vec<_>>()
        .as_slice()
    {
        [t, p] => (p.to_string(), t.to_string()),
        _ => return None,
    };

    let (cnt, tot) = match &progress_and_pkg_trp[0]
        .splitn(2, "/")
        .filter_map(|s| s.parse::<u64>().ok())
        .collect::<Vec<_>>()
        .as_slice()
    {
        [cnt, tot] => (*cnt, *tot),
        _ => (0, 0),
    };

    Some((pkg, triplet, cnt, tot))
}
//    Building [==============================================> ] 58/59
