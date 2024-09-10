use std::{fs, process, str};

#[derive(Debug)]
struct Package<'a> {
    name: &'a str,
    tags: Vec<&'a str>,
}

fn main() {
    let kurfile = fs::read_to_string("kurfile").expect("read fail");
    let packages = get_packages(&kurfile);

    println!(">>> Installing Platform Packages");
    let ostype = os_type::current_platform().os_type;
    match ostype {
        os_type::OSType::Ubuntu => {
            println!(">>> Platform is {ostype:?}");
            install_ubuntu(&packages);
        }
        _ => {
            println!("Unsupported platform {ostype:?}");
            process::exit(0);
        }
    }
    println!("");

    println!(">>> Installing Cargo Packages");
    install_cargo(&packages);
    println!("");

    println!(">>> All Good");
}

fn install_cargo(packages: &[Package]) {
    let platform_packages = packages.iter()
        .filter(|p| p.tags.contains(&"#cargo"));

    let args = platform_packages
        .map(|p| p.name);
    process::Command::new("cargo")
        .arg("install")
        .args(args)
        .status()
        .expect("install fail");
}

fn install_ubuntu(packages: &[Package]) {
    let platform_packages = packages.iter()
        .filter(|p| p.tags.contains(&"#ubuntu"));

    let args = platform_packages
        .map(|p| p.name);
    let apt = process::Command::new("sudo")
        .args(["apt", "install"])
        .args(args)
        .output()
        .expect("install fail");

    let stdout = str::from_utf8(&apt.stdout)
        .expect("invalid utf-8 output");
    let stdout_lines = stdout
        .split('\n')
        .map(|l| l.trim())
        .filter(|l|
            !l.contains("is already the newest version")
            &&
            !l.is_empty()
        );
    for line in stdout_lines {
        println!("{}", line.trim());
    }
}

fn get_packages<'a>(kurfile: &'a str) -> Vec<Package<'a>> {
    let lines = kurfile
        .split("\n")
        .map(|l| l.trim())
        .filter(|l| !l.is_empty() && !l.starts_with('#'));

    let packages = lines
        .map(|l| l.split(' '))
        .map(|mut parts| Package {
            name: parts.next().unwrap(),
            tags: parts.filter(|p| p.starts_with('#')).collect(),
        });

    packages.collect()
}
