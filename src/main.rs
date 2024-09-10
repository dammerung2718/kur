use clap::Parser;
use std::{collections::HashMap, fmt, fs, process, str};

const KURFILE: &str = "kurfile";
const CARGO_TAG: &str = "#cargo";
const UBUNTU_TAG: &str = "#ubuntu";
const BREW_TAG: &str = "#brew";

#[derive(clap::Parser)]
#[command(version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(clap::Subcommand)]
enum Cmd {
    /// Install packages
    Sync,

    /// Format "kurfile"
    Fmt,
}

#[derive(Debug, Clone)]
struct Package<'a> {
    line_no: u64,
    name: &'a str,
    tags: Vec<&'a str>,
}

struct DuplicateError<'a> {
    first: &'a Package<'a>,
    second: &'a Package<'a>,
}

impl<'a> fmt::Display for DuplicateError<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "package '{}' is mentioned twice on lines {} and {}",
            self.first.name, self.first.line_no, self.second.line_no
        )
    }
}

fn main() {
    let kurfile = fs::read_to_string(KURFILE).expect("read fail");
    let packages = get_packages(&kurfile);
    if let Some(err) = check_packages(&packages) {
        println!("{err}");
        process::exit(1);
    }

    let cli = Cli::parse();
    match cli.cmd {
        Cmd::Fmt => fmt(&packages),
        Cmd::Sync => sync(&packages),
    }
}

fn fmt(packages: &[Package]) {
    let mut formatted: Vec<String> = Vec::new();

    let common: Vec<_> = packages.iter().filter(|p| p.tags.len() > 1).collect();
    if !common.is_empty() {
        let mut common = fmt_packages(&common);
        formatted.push("# Common Packages".into());
        formatted.append(&mut common);
        formatted.push("".into());
    }

    let ubuntu: Vec<_> = packages
        .iter()
        .filter(|p| p.tags.contains(&UBUNTU_TAG) && p.tags.len() == 1)
        .collect();
    if !ubuntu.is_empty() {
        let mut ubuntu = fmt_packages(&ubuntu);
        formatted.push("# Ubuntu Packages".into());
        formatted.append(&mut ubuntu);
        formatted.push("".into());
    }

    let brew: Vec<_> = packages
        .iter()
        .filter(|p| p.tags.contains(&BREW_TAG) && p.tags.len() == 1)
        .collect();
    if !brew.is_empty() {
        let mut brew = fmt_packages(&brew);
        formatted.push("# Brew Packages".into());
        formatted.append(&mut brew);
        formatted.push("".into());
    }

    let cargo: Vec<_> = packages
        .iter()
        .filter(|p| p.tags.contains(&CARGO_TAG) && p.tags.len() == 1)
        .collect();
    if !cargo.is_empty() {
        let mut cargo = fmt_packages(&cargo);
        formatted.push("# Cargo Packages".into());
        formatted.append(&mut cargo);
        formatted.push("".into());
    }

    let formatted = formatted.join("\n");
    fs::write(KURFILE, formatted).expect("write fail");
}

fn fmt_packages(packages: &[&Package]) -> Vec<String> {
    let mut longest = 0;
    for p in packages {
        longest = std::cmp::max(longest, p.name.len());
    }
    longest += 4;

    packages
        .iter()
        .map(|p| format!("{:width$}{}", p.name, p.tags.join(" "), width = longest))
        .collect::<Vec<_>>()
}

fn sync(packages: &[Package]) {
    let ostype = os_type::current_platform().os_type;

    println!(">>> Installing {ostype:?} Packages");
    install_platform_packages(ostype, packages);

    println!(">>> Installing Cargo Packages");
    install_cargo(packages);

    println!(">>> All Good");
}

fn install_platform_packages(ostype: os_type::OSType, packages: &[Package]) {
    match ostype {
        os_type::OSType::Ubuntu => {
            let ubuntu: Vec<_> = packages
                .iter()
                .filter(|p| p.tags.contains(&UBUNTU_TAG))
                .collect();
            if !ubuntu.is_empty() {
                install_ubuntu(&ubuntu);
            }
        }
        os_type::OSType::OSX => {
            let brew: Vec<_> = packages
                .iter()
                .filter(|p| p.tags.contains(&BREW_TAG))
                .collect();
            if !brew.is_empty() {
                install_brew(&brew);
            }
        }
        _ => {
            println!("Unsupported platform {ostype:?}");
            process::exit(1);
        }
    }
}

fn install_brew(packages: &[&Package]) {
    let packages = packages
        .iter()
        .filter(|p| p.tags.contains(&BREW_TAG))
        .map(|p| p.name);
    process::Command::new("brew")
        .arg("install")
        .args(packages)
        .stderr(process::Stdio::null())
        .status()
        .expect("install fail");
}

fn install_cargo(packages: &[Package]) {
    process::Command::new("cargo")
        .arg("install")
        .arg("cargo-binstall")
        .status()
        .expect("install fail");

    let packages = packages
        .iter()
        .filter(|p| p.tags.contains(&CARGO_TAG))
        .map(|p| p.name);
    process::Command::new("cargo")
        .arg("binstall")
        .arg("--strategies")
        .arg("crate-meta-data")
        .args(packages)
        .status()
        .expect("install fail");
}

fn install_ubuntu(packages: &[&Package]) {
    let packages = packages
        .iter()
        .filter(|p| p.tags.contains(&UBUNTU_TAG))
        .map(|p| p.name);
    process::Command::new("sudo")
        .args(["apt", "install"])
        .args(packages)
        .stderr(process::Stdio::null())
        .status()
        .expect("install fail");
}

fn check_packages<'a>(packages: &'a [Package]) -> Option<DuplicateError<'a>> {
    check_duplicates(packages)
}

fn check_duplicates<'a>(packages: &'a [Package]) -> Option<DuplicateError<'a>> {
    let mut seen: HashMap<&str, &Package> = HashMap::new();

    for p in packages {
        if let Some(last_seen) = seen.get(&p.name) {
            return Some(DuplicateError {
                first: last_seen,
                second: p,
            });
        }
        seen.insert(p.name, p);
    }

    None
}

fn get_packages(kurfile: &str) -> Vec<Package> {
    let mut packages: Vec<_> = kurfile
        .split('\n')
        .enumerate()
        .map(|(n, l)| (n, l.trim()))
        .filter(|(_, l)| !l.is_empty() && !l.starts_with('#'))
        .map(|(n, l)| (n, l.split(' ')))
        .map(|(n, mut parts)| Package {
            line_no: n as u64,
            name: parts.next().unwrap(),
            tags: parts.filter(|p| p.starts_with('#')).collect(),
        })
        .collect();

    // sort packages alphabetically
    packages.sort_by(|a, b| a.name.cmp(b.name));

    // sort tags alphabetically
    for p in &mut packages {
        p.tags.sort();
    }

    // done
    packages
}
