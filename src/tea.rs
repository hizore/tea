use reqwest::Client;
use serde::Deserialize;
use std::io::{self, Write};
use std::path::Path;
use std::process::{Command, ExitStatus};
use colored::*;

#[derive(Deserialize)]
struct Repository {
    full_name: String,
    html_url: String,
}

#[derive(Deserialize)]
struct SearchResult {
    items: Vec<Repository>,
}

async fn search_repositories(query: &str) -> Result<Vec<Repository>, reqwest::Error> {
    let url = format!("https://api.github.com/search/repositories?q={}&per_page=10", query);
    let client = Client::new();
    let res = client
        .get(&url)
        .header("User-Agent", "rust-github-client")
        .send()
        .await?
        .json::<SearchResult>()
        .await?;
    Ok(res.items)
}

fn choose_repository(repos: &[Repository]) -> Option<&Repository> {
    for (i, repo) in repos.iter().enumerate() {
        println!("{}: {} - {}", i + 1, repo.full_name, repo.html_url);
    }

    println!("{}", "|------------------------------------------------------------------------------------------------------|".red());
    println!("{}", " If compilation does not occur or gives an error, go to the repo page and read the description.".red());
    println!("{}", " Btw repo that u trying to clone can be NOT C/CPP/RUST project so READ THE FUCKING REPO PAGE".red());
    println!("{}", " Ubuntu - worst distro ever.".red());
    println!("{}", "|------------------------------------------------------------------------------------------------------|".red());

    print!("Enter the number of the repository to download: ");
    io::stdout().flush().unwrap();

    let mut input = String::new();
    io::stdin().read_line(&mut input).unwrap();
    let choice = input.trim().parse::<usize>().ok()?;

    repos.get(choice - 1)
}

fn download_repository(repo: &Repository) -> io::Result<()> {
    let url = format!("https://github.com/{}.git", repo.full_name);
    println!("Cloning repository from {}", url);
    Command::new("git")
        .arg("clone")
        .arg(url)
        .status()?;
    Ok(())
}

fn configure_cmake(path: &str) -> io::Result<ExitStatus> {
    let build_path = format!("{}/build", path);
    std::fs::create_dir_all(&build_path)?;
    Command::new("cmake")
        .args(&["..", "--fresh"])
        .current_dir(&build_path)
        .status()
}

fn build_with_cmake(path: &str) -> io::Result<ExitStatus> {
    Command::new("cmake")
        .args(&["--build", ".", "--parallel"])
        .current_dir(format!("{}/build", path))
        .status()
}

fn build_with_make(path: &str) -> io::Result<ExitStatus> {
    Command::new("make").current_dir(path).status()
}

fn configure_meson(path: &str) -> io::Result<ExitStatus> {
    Command::new("meson")
        .args(&["setup", "build"])
        .current_dir(path)
        .status()
}

fn build_with_meson(path: &str) -> io::Result<ExitStatus> {
    Command::new("meson")
        .arg("compile")
        .current_dir(format!("{}/build", path))
        .status()
}

fn build_with_cargo(path: &str) -> io::Result<ExitStatus> {
    Command::new("cargo")
        .arg("build")
        .current_dir(path)
        .status()
}

fn find_build_systems(path: &str) -> Vec<&'static str> {
    [
        ("CMakeLists.txt", "CMake"),
        ("Makefile", "Make"),
        ("meson.build", "Meson"),
        ("Cargo.toml", "Cargo"),
    ]
    .iter()
    .filter_map(|(file, system)| {
        Path::new(&format!("{}/{}", path, file)).exists().then_some(*system)
    })
    .collect()
}


fn choose_build_system<'a>(systems: &'a [&'a str]) -> Option<&'a str> {
    for (i, system) in systems.iter().enumerate() {
        println!("{}: {}", i + 1, system);
    }
    print!("Enter the number of the build system to use: ");
    io::stdout().flush().unwrap();

    let mut input = String::new();
    io::stdin().read_line(&mut input).unwrap();
    let choice = input.trim().parse::<usize>().ok()?;

    systems.get(choice - 1).copied()
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    print!("Enter search query: ");
    io::stdout().flush()?;

    let mut query = String::new();
    io::stdin().read_line(&mut query)?;

    let repos = search_repositories(query.trim()).await?;
    let repo = choose_repository(&repos).ok_or("No repository chosen")?;
    let repo_name = repo.full_name.split('/').last().unwrap();

    download_repository(&repo)?;
    let build_systems = find_build_systems(repo_name);
    if build_systems.is_empty() {
        return Err("No build systems found".into());
    }

    let system = choose_build_system(&build_systems).ok_or("No build system chosen")?;
    let status = match system {
        "CMake" => {
            configure_cmake(repo_name)?;
            build_with_cmake(repo_name)?
        }
        "Make" => build_with_make(repo_name)?,
        "Meson" => {
            configure_meson(repo_name)?;
            build_with_meson(repo_name)?
        }
        "Cargo" => {
            build_with_cargo(repo_name)?;
            Command::new("cargo")
                .arg("run")
                .current_dir(repo_name)
                .status()?
        }
        _ => return Err("Unsupported build system".into()),
    };

    println!("{}", if status.success() { "Build completed successfully!" } 
        else 
        { "Build failed." });
    Ok(())
}
