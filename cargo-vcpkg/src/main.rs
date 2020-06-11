use anyhow::{bail, Context};
//use indicatif::{ProgressBar, ProgressStyle};
use serde::Deserialize;
use std::{
    collections::BTreeMap,
    fs::File,
    io::{BufRead, BufReader, Cursor, Write},
    process::{Command, Output, Stdio},
    str,
    time::SystemTime,
};
use structopt::StructOpt;
use termcolor::{Color, ColorChoice, ColorSpec, StandardStream, WriteColor};
use vcpkg::{find_vcpkg_root, Config};

// settings for a specific Rust target
#[serde(rename_all = "kebab-case")]
#[derive(Debug, Deserialize)]
struct Target {
    triplet: Option<String>,
    // this dependencies key for a specific target overrides the main entry
    // so a the target can opt out of installing packages
    #[serde(alias = "install")]
    dependencies: Option<Vec<String>>,
    dev_dependencies: Option<Vec<String>>,
}

#[serde(rename_all = "kebab-case")]
#[derive(Debug, Deserialize)]
struct Vcpkg {
    //  vcpkg_root: Option<String>,
    #[serde(default = "BTreeMap::new")]
    target: BTreeMap<String, Target>,
    branch: Option<String>,
    rev: Option<String>,
    git: Option<String>,
    tag: Option<String>,

    #[serde(alias = "install")]
    dependencies: Option<Vec<String>>,
    dev_dependencies: Option<Vec<String>>,
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
    /// Display more information while building
    ///
    /// This will display the output of git and
    /// vcpkg operations.
    #[structopt(short, long)]
    verbose: bool,

    // #[structopt(long)]
    // manifest_path: Option<String>,
    #[structopt(subcommand)]
    sub: Subcommands,
}

#[derive(Debug, PartialEq, StructOpt)]
enum Subcommands {
    /// Build packages, checking out the correct version
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

enum RevSelector {
    Rev(String),
    Tag(String),
    Branch(String),
}

fn main() {
    // cargo passes us the "vcpkg" arg when it calls us. Drop it before
    // parsing the arg list so it doesn't end up the usage description
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

fn build(opt: Opt) -> Result<(), anyhow::Error> {
    let start_time = SystemTime::now();

    let target_triple = target_triple();

    let verbose = opt.verbose;

    let mut args = std::env::args().skip_while(|val| !val.starts_with("--manifest-path"));
    let mut cmd = cargo_metadata::MetadataCommand::new();

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

    let (git_url, vcpkg_ports, rev_tag_branch, vcpkg_triplet, root_crate) =
        process_metadata(&metadata, &target_triple)?;

    // should we modify the existing?
    // let mut allow_updates = true;

    // find the vcpkg root
    let vcpkg_root = find_vcpkg_root(&Config::default()).unwrap_or_else(|_| {
        let target_directory = metadata.target_directory.clone();
        let mut vcpkg_root = target_directory;
        vcpkg_root.push("vcpkg");
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
            "could not find a vcpkg installation and crate \n\
        {} does not specify a git repository to clone from. \n\n\
        Add a [package.metadata.vcpkg] section to the root crate's\n\
        Cargo.toml, and add a 'git' key and one of the 'branch',\n\
        'tag' or 'rev' keys to tell this program where to get\n\
        the correct version of vcpkg from. For example:\n\n\
        [package.metadata.vcpkg]\n\
        git = \"https://github.com/microsoft/vcpkg\"\n\
        branch = \"master\" ",
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

    // check out the required rev
    let rev_tag_branch = rev_tag_branch.unwrap();
    let (desc, rev_tag_branch, do_pull) = match rev_tag_branch {
        RevSelector::Rev(r) => ("rev", r, false),
        RevSelector::Tag(t) => ("tag", t, false), //?
        RevSelector::Branch(b) => ("branch", b, true),
    };
    print_tag("Checkout", &format!("{} {}", desc, rev_tag_branch));
    let mut cmd = Command::new("git");
    cmd.arg("checkout");
    cmd.arg(&rev_tag_branch);
    cmd.current_dir(&vcpkg_root);
    run_command(cmd, verbose).context("failed to execute process")?;

    // if it is a branch, run a git pull to move to the correct commit
    if do_pull {
        print_tag("Pulling", &format!("{} {}", desc, rev_tag_branch));
        let mut cmd = Command::new("git");
        cmd.arg("pull");
        //cmd.arg(rev_tag_branch);
        cmd.current_dir(&vcpkg_root);
        run_command(cmd, verbose).context("failed to execute process")?;
    }
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
            !output.status.success()
                || str::from_utf8(&output.stdout)
                    .unwrap()
                    .contains("Warning: Different source is available for vcpkg")
        }
        Err(_) => true,
    };

    if require_bootstrap {
        run_bootstrap(&vcpkg_root, verbose)?;
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
        if let Some((pkg, triplet, _num, _tot)) = parse_build_line(&line) {
            print_tag("Compiling", &format!("{} (triplet {})", pkg, triplet))
        }

        if verbose {
            println!("{}", line);
        }
        // bar.set_length(tot);
        // bar.set_position(num);
        //    Building [==============================================> ] 58/59
    }
    // grab anything that is left
    let output = output.wait_with_output()?;

    if !output.status.success() {
        if !verbose {
            println!("-- stdout --\n{}", String::from_utf8_lossy(&output.stdout));
            println!("-- stderr --\n{}", String::from_utf8_lossy(&output.stderr));
        }
        bail!("failed");
    }

    let duration = SystemTime::now().duration_since(start_time).unwrap();
    print_tag("Finished", &format!("in {:0.2}s", duration.as_secs_f32()));
    Ok(())
}

fn process_metadata(
    metadata: &cargo_metadata::Metadata,
    target_triple: &str,
) -> Result<
    (
        Option<String>,
        Vec<String>,
        Option<RevSelector>,
        Option<String>,
        cargo_metadata::PackageId,
    ),
    anyhow::Error,
> {
    let resolve = metadata.resolve.as_ref().unwrap();
    let root_crate = resolve
        .root
        .as_ref()
        .context("cannot run on a virtual manifest, this command requires running against an actual package in this workspace.")?;

    let mut git_url = None;
    let mut vcpkg_ports = Vec::new();
    let mut rev_tag_branch: Option<RevSelector> = None;
    let mut vcpkg_triplet = None;
    for p in &metadata.packages {
        // dbg!(&p);
        if let Ok(v) = serde_json::from_value::<Metadata>(p.metadata.clone()) {
            // dbg!(&v);
            let v = v.vcpkg;
            let is_root_crate = p.id == *root_crate;

            // only use git url and rev from the root crate
            if v.git.is_some() && is_root_crate {
                git_url = v.git;

                // TODO: check the target and use it's package set if required
                // TODO: get the correct target
                // TODO: make sure to pull if it's a branch
                rev_tag_branch = match (&v.branch, &v.tag, &v.rev) {
                    (Some(b), None, None) => Some(RevSelector::Branch(b.into())),
                    (None, Some(t), None) => Some(RevSelector::Tag(t.into())),
                    (None, None, Some(r)) => Some(RevSelector::Rev(r.into())),
                    _ => {
                        bail!("must specify one of branch,rev,tag for git source");
                    }
                };
            }

            // if there is specific configuration for the target and it has
            // a dependencies key, use that rather than the general dependencies key
            match v.target.get(target_triple) {
                Some(target) => {
                    if target.dependencies.is_some() {
                        vcpkg_ports
                            .extend_from_slice(&target.dependencies.as_ref().unwrap().as_slice());
                    } else {
                        if v.dependencies.is_some() {
                            vcpkg_ports
                                .extend_from_slice(&v.dependencies.as_ref().unwrap().as_slice());
                        }
                    }
                    if is_root_crate && target.triplet.is_some() {
                        vcpkg_triplet = target.triplet.clone();
                    }
                    if is_root_crate && target.dev_dependencies.is_some() {
                        vcpkg_ports.extend_from_slice(
                            &target.dev_dependencies.as_ref().unwrap().as_slice(),
                        );
                    }
                }
                _ => {
                    // not found or dependencies is empty
                    if v.dependencies.is_some() {
                        vcpkg_ports.extend_from_slice(&v.dependencies.as_ref().unwrap().as_slice());
                    }
                    if is_root_crate && v.dev_dependencies.is_some() {
                        vcpkg_ports
                            .extend_from_slice(&v.dev_dependencies.as_ref().unwrap().as_slice());
                    }
                }
            }
        }
    }
    Ok((
        git_url,
        vcpkg_ports,
        rev_tag_branch,
        vcpkg_triplet,
        root_crate.clone(),
    ))
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

    if !output.status.success() {
        if !verbose {
            println!("-- stdout --\n{}", String::from_utf8_lossy(&output.stdout));
            println!("-- stderr --\n{}", String::from_utf8_lossy(&output.stderr));
        }
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

    let progress_and_pkg_trp = line.splitn(2, ':').collect::<Vec<_>>();
    if progress_and_pkg_trp.len() != 2 {
        return None;
    }

    let pkg_with_triplet = progress_and_pkg_trp[1].trim();

    let (pkg, triplet) = match pkg_with_triplet
        .rsplitn(2, ':')
        .collect::<Vec<_>>()
        .as_slice()
    {
        [t, p] => (p.to_string(), t.to_string()),
        _ => return None,
    };

    let (cnt, tot) = match &progress_and_pkg_trp[0]
        .splitn(2, '/')
        .filter_map(|s| s.parse::<u64>().ok())
        .collect::<Vec<_>>()
        .as_slice()
    {
        [cnt, tot] => (*cnt, *tot),
        _ => (0, 0),
    };

    Some((pkg, triplet, cnt, tot))
}

fn run_bootstrap(vcpkg_root: &std::path::Path, verbose: bool) -> Result<(), anyhow::Error> {
    print_tag("Compiling", "vcpkg");

    if cfg!(windows) {
        let mut cmd = Command::new("cmd");
        cmd.arg("/C");
        cmd.arg("bootstrap-vcpkg.bat");
        cmd.arg("-disableMetrics");
        cmd.current_dir(&vcpkg_root);
        run_command(cmd, verbose).context("failed to run vcpkg bootstrap")?;
    } else {
        // if we are on a mac with clang 11, try it first. Fall back to the
        // installation that requires gcc if this build fails
        if cfg!(target_os = "macos") {
            if let Some(version) = apple_clang_version() {
                if version >= 11 {
                    let mut cmd = Command::new("sh");
                    cmd.arg("-c");
                    cmd.arg("./bootstrap-vcpkg.sh -disableMetrics -allowAppleClang");
                    cmd.current_dir(&vcpkg_root);
                    if run_command(cmd, verbose).is_ok() {
                        return Ok(());
                    }
                    println!(
                        "note: building vcpkg with apple clang failed, falling \
                    back to using another compiler."
                    );
                }
            }
        }

        let mut cmd = Command::new("sh");
        cmd.arg("-c");
        cmd.arg("./bootstrap-vcpkg.sh -disableMetrics");
        cmd.current_dir(&vcpkg_root);
        run_command(cmd, verbose).context("failed to run vcpkg bootstrap")?;
    };

    Ok(())
}

fn apple_clang_version() -> Option<u64> {
    let output = Command::new("clang").arg("--version").output().ok()?;
    parse_apple_clang_version(&output.stdout)
}

fn parse_apple_clang_version(bytes: &[u8]) -> Option<u64> {
    Cursor::new(bytes)
        .lines()
        .filter_map(Result::ok)
        .filter(|line| line.starts_with("Apple clang version "))
        .map(|line| line.trim_start_matches("Apple clang version ").to_string())
        .next()?
        .splitn(2, '.')
        .next()
        .and_then(|x| x.parse::<u64>().ok())
}

#[cfg(test)]
mod test {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::{
        env, fs,
        path::{Path, PathBuf},
    };
    // Cadged from https://github.com/rust-lang/cargo/tree/master/crates/cargo-test-support
    #[derive(Default)]
    pub(super) struct ProjectBuilder {
        root: PathBuf,
        //  files:
    }
    pub(super) fn project() -> ProjectBuilder {
        static NEXT_ID: AtomicUsize = AtomicUsize::new(0);
        let id = NEXT_ID.fetch_add(1, Ordering::Relaxed);

        println!("new id {}", id);

        let mut root = {
            let mut path = env::current_exe().unwrap();
            //            dbg!(&path);
            path.pop(); // chop off exe name
            path.pop(); // chop off deps
            path.pop(); // chop off 'debug'

            // if path.file_name().and_then(|s| s.to_str()) != Some("target") {
            //     path.pop();
            // }

            path.push("cv_int");
            path.mkdir_p();
            path
        };

        root.push(&format!("test_{}", id));
        root.rm_rf();
        root.mkdir_p();

        ProjectBuilder { root }
    }

    impl ProjectBuilder {
        // pub(super) fn file(&mut self, name: &str, contents: &str) -> &mut Self {
        //     self
        // }

        pub(super) fn metadata(
            &mut self,
            manifest_path: &str,
        ) -> Result<cargo_metadata::Metadata, anyhow::Error> {
            let mut cmd = cargo_metadata::MetadataCommand::new();

            let mut path = self.root.clone();
            path.push(manifest_path);
            cmd.manifest_path(path);

            Ok(cmd.exec()?)
        }

        /// Adds a file to the project.
        pub(super) fn file<B: AsRef<Path>>(self, path: B, body: &str) -> Self {
            let path = self.root.clone().join(path);
            let dirname = path.parent().unwrap();
            dirname.mkdir_p();
            fs::write(&path, &body)
                .unwrap_or_else(|e| panic!("could not create file {}: {}", path.display(), e));

            self
        }
    }

    pub trait TestPathExt {
        fn rm_rf(&self);
        fn mkdir_p(&self);
    }

    impl TestPathExt for Path {
        fn rm_rf(&self) {
            if self.exists() {
                if let Err(e) = remove_dir_all::remove_dir_all(self) {
                    panic!("failed to remove {:?}: {:?}", self, e)
                }
            }
        }

        fn mkdir_p(&self) {
            fs::create_dir_all(self)
                .unwrap_or_else(|e| panic!("failed to mkdir_p {}: {}", self.display(), e))
        }
    }
    pub fn basic_manifest(name: &str, version: &str) -> String {
        format!(
            r#"
            [package]
            name = "{}"
            version = "{}"
            authors = []
        "#,
            name, version
        )
    }
    pub fn extended_manifest(name: &str, version: &str, tail: &str) -> String {
        format!("{}\n\n{}", &basic_manifest(name, version), tail)
    }

    #[test]
    fn test_parse_apple_clang_version() {
        assert_eq!(
            parse_apple_clang_version(b"la la la\nApple clang version 9.0.1"),
            Some(9)
        );
        assert_eq!(
            parse_apple_clang_version(b"ho ho ho\nhe he he\nApple clang version 10.0.1"),
            Some(10)
        );
        assert_eq!(
            parse_apple_clang_version(b"Apple clang version 11.0.1"),
            Some(11)
        );
        assert_eq!(
            parse_apple_clang_version(b"Apple clang version 12.0.1"),
            Some(12)
        );
        assert_eq!(
            parse_apple_clang_version(b"Opple clong version 12.0.1"),
            None
        );
    }

    #[test]
    fn run_on_workspace_fails() {
        let metadata = test::project()
            .file(
                "Cargo.toml",
                r#"
                    [workspace]
                    members = ["top", "dep"]
                "#,
            )
            .file(
                "top/Cargo.toml",
                &extended_manifest(
                    "top",
                    "0.1.0",
                    r#"
                        [dependencies]
                        dep = { path = "../dep" }
                        [package.manifest.vcpkg]
                        dependencies = ["z85"]
                    "#,
                ),
            )
            .file("top/src/main.rs", "")
            .file(
                "dep/Cargo.toml",
                &extended_manifest(
                    "dep",
                    "0.1.0",
                    r#"
                [lib]
                [package.manifest.vcpkg]
            "#,
                ),
            )
            .file("dep/src/lib.rs", "")
            .metadata("Cargo.toml")
            .unwrap();
        let err = process_metadata(&metadata, "").err().unwrap();
        assert!(err.to_string().contains("cannot run on a virtual manifest"));
    }

    #[test]
    fn install_in_root_crate() {
        let metadata = test::project()
            .file(
                "Cargo.toml",
                r#"
                    [workspace]
                    members = ["top", "dep"]
                "#,
            )
            .file(
                "top/Cargo.toml",
                &extended_manifest(
                    "top",
                    "0.1.0",
                    r#"
                        [dependencies]
                        dep = { path = "../dep" }
                        [package.metadata.vcpkg]
                        install = ["z85"]
                    "#,
                ),
            )
            .file("top/src/main.rs", "")
            .file(
                "dep/Cargo.toml",
                &extended_manifest(
                    "dep",
                    "0.1.0",
                    r#"
                [lib]
                [package.metadata.vcpkg]
            "#,
                ),
            )
            .file("dep/src/lib.rs", "")
            .metadata("top/Cargo.toml")
            .unwrap();

        let (_, vcpkg_ports, _, _, _) = process_metadata(&metadata, "").unwrap();

        assert_eq!(vcpkg_ports, vec!["z85"]);
    }
    #[test]
    fn same_dependencies_but_specified_triplet() {
        let metadata = test::project()
            .file(
                "Cargo.toml",
                r#"
                    [workspace]
                    members = ["top", "dep"]
                "#,
            )
            .file(
                "top/Cargo.toml",
                &extended_manifest(
                    "top",
                    "0.1.0",
                    r#"
                        [dependencies]
                        dep = { path = "../dep" }
                        [package.metadata.vcpkg]
                        install = ["z85"]
                        [package.metadata.vcpkg.target]
                        x86_64-pc-windows-msvc = { triplet = "x64-windows-static-md" } 
                    "#,
                ),
            )
            .file("top/src/main.rs", "")
            .file(
                "dep/Cargo.toml",
                &extended_manifest(
                    "dep",
                    "0.1.0",
                    r#"
                [lib]
                [package.metadata.vcpkg]
            "#,
                ),
            )
            .file("dep/src/lib.rs", "")
            .metadata("top/Cargo.toml")
            .unwrap();

        let (_, vcpkg_ports, _, vcpkg_triplet, _) =
            process_metadata(&metadata, "x86_64-pc-windows-msvc").unwrap();

        assert_eq!(vcpkg_ports, vec!["z85"]);
        assert_eq!(vcpkg_triplet, Some("x64-windows-static-md".to_owned()));
    }

    #[test]
    fn specified_triplet_requires_no_dependencies() {
        let metadata = test::project()
            .file(
                "Cargo.toml",
                r#"
                    [workspace]
                    members = ["top", "dep"]
                "#,
            )
            .file(
                "top/Cargo.toml",
                &extended_manifest(
                    "top",
                    "0.1.0",
                    r#"
                        [dependencies]
                        dep = { path = "../dep" }
                        [package.metadata.vcpkg]
                        install = ["z85"]
                        [package.metadata.vcpkg.target]
                        x86_64-pc-windows-msvc = { triplet = "x64-windows-static-md", dependencies = [] } 
                    "#,
                ),
            )
            .file("top/src/main.rs", "")
            .file(
                "dep/Cargo.toml",
                &extended_manifest(
                    "dep",
                    "0.1.0",
                    r#"
                [lib]
                [package.metadata.vcpkg]
            "#,
                ),
            )
            .file("dep/src/lib.rs", "")
            .metadata("top/Cargo.toml")
            .unwrap();

        let (_, vcpkg_ports, _, vcpkg_triplet, _) =
            process_metadata(&metadata, "x86_64-pc-windows-msvc").unwrap();

        assert_eq!(vcpkg_ports, Vec::<String>::new());
        assert_eq!(vcpkg_triplet, Some("x64-windows-static-md".to_owned()));
    }

    #[test]
    fn combine_deps_from_all_crates() {
        let metadata = test::project()
            .file(
                "Cargo.toml",
                r#"
                    [workspace]
                    members = ["top", "dep"]
                "#,
            )
            .file(
                "top/Cargo.toml",
                &extended_manifest(
                    "top",
                    "0.1.0",
                    r#"
                        [dependencies]
                        dep = { path = "../dep" }
                        [package.metadata.vcpkg]
                        dependencies = ["a"]
                        dev-dependencies = ["d"]
                        [package.metadata.vcpkg.target]
                        x86_64-pc-windows-msvc = { triplet = "x64-windows-static-md", dev-dependencies = ["b", "c"] } 
                    "#,
                ),
            )
            .file("top/src/main.rs", "")
            .file(
                "dep/Cargo.toml",
                &extended_manifest(
                    "dep",
                    "0.1.0",
                    r#"
                [lib]
                [package.metadata.vcpkg]
                dependencies = ["m"]
                dev-dependencies = ["n"]
                [package.metadata.vcpkg.target]
                x86_64-pc-windows-msvc = { triplet = "x64-windows-static-md", dependencies = ["o"], dev-dependencies = ["p"] } 
            "#,
                ),
            )
            .file("dep/src/lib.rs", "")
            .metadata("top/Cargo.toml")
            .unwrap();

        let (_, mut vcpkg_ports, _, vcpkg_triplet, _) =
            process_metadata(&metadata, "x86_64-pc-windows-msvc").unwrap();
        vcpkg_ports.sort();
        assert_eq!(vcpkg_ports, vec!["a", "b", "c", "o"]);
        assert_eq!(vcpkg_triplet, Some("x64-windows-static-md".to_owned()));

        let (_, mut vcpkg_ports, _, _, _) = process_metadata(&metadata, "").unwrap();
        vcpkg_ports.sort();
        assert_eq!(vcpkg_ports, vec!["a", "d", "m"]);
    }
}
