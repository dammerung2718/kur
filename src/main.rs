use std::{fs, process, str};

use clap::Parser;

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
    name: &'a str,
    tags: Vec<&'a str>,
}

fn main() {
    let cli = Cli::parse();
    match cli.cmd {
        Cmd::Fmt => fmt(),
        Cmd::Sync => sync(),
    }
}

fn fmt() {
    let kurfile = fs::read_to_string("kurfile").expect("read fail");
    let packages = get_packages(&kurfile);

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
        .filter(|p| p.tags.contains(&"#ubuntu") && p.tags.len() == 1)
        .collect();
    if !ubuntu.is_empty() {
        let mut ubuntu = fmt_packages(&ubuntu);
        formatted.push("# Ubuntu Packages".into());
        formatted.append(&mut ubuntu);
        formatted.push("".into());
    }

    let brew: Vec<_> = packages
        .iter()
        .filter(|p| p.tags.contains(&"#brew") && p.tags.len() == 1)
        .collect();
    if !brew.is_empty() {
        let mut brew = fmt_packages(&brew);
        formatted.push("# Brew Packages".into());
        formatted.append(&mut brew);
        formatted.push("".into());
    }

    let cargo: Vec<_> = packages
        .iter()
        .filter(|p| p.tags.contains(&"#cargo") && p.tags.len() == 1)
        .collect();
    if !cargo.is_empty() {
        let mut cargo = fmt_packages(&cargo);
        formatted.push("# Cargo Packages".into());
        formatted.append(&mut cargo);
        formatted.push("".into());
    }

    let formatted = formatted.join("\n");
    fs::write("kurfile", formatted).expect("write fail");
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

fn sync() {
    let kurfile = fs::read_to_string("kurfile").expect("read fail");
    let packages = get_packages(&kurfile);
    let ostype = os_type::current_platform().os_type;

    println!(">>> Installing {ostype:?} Packages");
    install_platform_packages(ostype, &packages);

    println!(">>> Installing Cargo Packages");
    install_cargo(&packages);

    println!(">>> All Good");
}

fn install_platform_packages(ostype: os_type::OSType, packages: &[Package]) {
    match ostype {
        os_type::OSType::Ubuntu => {
            let ubuntu: Vec<_> = packages
                .iter()
                .filter(|p| p.tags.contains(&"#ubuntu"))
                .collect();
            if !ubuntu.is_empty() {
                install_ubuntu(&ubuntu);
            }
        }
        os_type::OSType::OSX => {
            let brew: Vec<_> = packages
                .iter()
                .filter(|p| p.tags.contains(&"#bre"))
                .collect();
            if !brew.is_empty() {
                install_brew(&brew);
            }
        }
        _ => {
            println!("Unsupported platform {ostype:?}");
            process::exit(0);
        }
    }
}

fn install_brew(packages: &[&Package]) {
    let packages = packages
        .iter()
        .filter(|p| p.tags.contains(&"#brew"))
        .map(|p| p.name);
    process::Command::new("brew")
        .arg("install")
        .args(packages)
        .stderr(process::Stdio::null())
        .status()
        .expect("install fail");
}

fn install_cargo(packages: &[Package]) {
    let packages = packages
        .iter()
        .filter(|p| p.tags.contains(&"#cargo"))
        .map(|p| p.name);
    process::Command::new("cargo")
        .arg("install")
        .args(packages)
        .status()
        .expect("install fail");
}

fn install_ubuntu(packages: &[&Package]) {
    let packages = packages
        .iter()
        .filter(|p| p.tags.contains(&"#ubuntu"))
        .map(|p| p.name);
    let apt = process::Command::new("sudo")
        .args(["apt", "install"])
        .args(packages)
        .output()
        .expect("install fail");

    let stdout = str::from_utf8(&apt.stdout).expect("invalid utf-8 output");
    let stdout_lines = stdout
        .split('\n')
        .map(|l| l.trim())
        .filter(|l| !l.contains("is already the newest version") && !l.is_empty());
    for line in stdout_lines {
        println!("{}", line.trim());
    }
}

fn get_packages(kurfile: &str) -> Vec<Package> {
    let mut packages: Vec<_> = kurfile
        .split('\n')
        .map(|l| l.trim())
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
        .map(|l| l.split(' '))
        .map(|mut parts| Package {
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
