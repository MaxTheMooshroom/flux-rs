use std::process::{Command, Output};

fn get_rust_toolchain() -> String {
    #[derive(serde::Deserialize)]
    pub struct ToolchainToml {
        toolchain: ToolchainSpec,
    }

    #[derive(serde::Deserialize)]
    pub struct ToolchainSpec {
        channel: String,
    }

    let toolchain_str = include_str!("../../rust-toolchain.toml");
    let toolchain_file: ToolchainToml = toml::from_str(toolchain_str)
        .expect("Failed to parse channel from rust-toolchain.toml");
    toolchain_file.toolchain.channel
}

fn get_rust_toolchain_commit_info(toolchain: &str) -> String {
    use std::{env, str::FromStr};

    use current_platform::CURRENT_PLATFORM;
    use rustup_toolchain_manifest::{InstallSpec, Manifest, Toolchain};

    const VERSION_OVERRIDE: &str = "FLUX_TOOLCHAIN_CARGO_VERSION_OVERRIDE";
    const SANDBOXED: &str = "SANDBOXED";

    match (env::var_os(SANDBOXED), env::var_os(VERSION_OVERRIDE)) {
        (Some(_), Some(version)) => version.into_string().unwrap(),
        _ => {
            let manifest_url = Toolchain::from_str(toolchain)
                .unwrap_or_else(|_| panic!("Invalid toolchain string: {}", toolchain))
                .manifest_url();

            let manifest_txt = reqwest::blocking::get(manifest_url)
                .expect("Failed to fetch manifest")
                .text()
                .expect("Failed to decode manifest");
            let manifest = Manifest::try_from(manifest_txt.as_str())
                .expect("Failed to parse manifest");

            let targets = std::collections::HashSet::new();
            let components = {
                let mut set = targets.clone();
                set.insert("cargo".to_string());
                set
            };

            let platform = platforms::Platform::find(&CURRENT_PLATFORM)
                .unwrap_or_else(|| panic!("Failed to find current platform: {}", CURRENT_PLATFORM));
            let spec = InstallSpec {
                profile: "minimal".to_string(),
                components,
                targets,
            };

            let downloads = manifest.find_downloads_for_install(&platform, &spec)
                .expect("Could not find downloads for current platform");

            let cargo = downloads.iter()
                .find(|pkg| pkg.name == "cargo")
                .expect("Could not find cargo for current platform");

            cargo.version
                .split_once(" ").unwrap().1
                .to_string()
        }
    }
}

fn parse_output(output: Option<Output>) -> Option<String> {
    Some(String::from_utf8(output?.stdout).ok()?.trim().to_string())
}

fn git_sha() -> String {
    parse_output(
        Command::new("git")
            .args(["describe", "--always", "--dirty=*"])
            .output()
            .ok(),
    )
    .unwrap_or("unknown".to_string())
}

fn git_sha_full() -> String {
    parse_output(
        Command::new("git")
            .args(["describe", "--always", "--abbrev=0", "--dirty=*"])
            .output()
            .ok(),
    )
    .unwrap_or("unknown".to_string())
}

fn git_date() -> String {
    parse_output(
        Command::new("git")
            .args(["show", "-s", "--format=%cd", "--date=format:%Y-%m-%d", "HEAD"])
            .output()
            .ok(),
    )
    .unwrap_or("unknown".to_string())
}

fn main() {
    println!("cargo:rustc-env=GIT_SHA={}", git_sha());
    println!("cargo:rustc-env=GIT_SHA_FULL={}", git_sha_full());
    println!("cargo:rustc-env=GIT_DATE={}", git_date());

    println!("cargo:rerun-if-env-changed=SANDBOXED");
    println!("cargo:rerun-if-env-changed=FLUX_TOOLCHAIN_CARGO_VERSION_OVERRIDE");

    let tc = get_rust_toolchain();
    let tc_cargo_version_str = get_rust_toolchain_commit_info(&tc);

    println!("cargo:rustc-env=RUST_TOOLCHAIN={}", tc);
    println!("cargo:rustc-env=RUST_TOOLCHAIN_CARGO_VERSION={}", tc_cargo_version_str);
}
