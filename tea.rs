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

    if choice > 0 && choice <= repos.len() {
        Some(&repos[choice - 1])
    } 
    else {
        None
    }
}

fn download_repository(repo: &Repository) -> ExitStatus {
    let url = format!("https://github.com/{}.git", repo.full_name);
    println!("Cloning repository from {}", url);
    Command::new("git")
        .arg("clone")
        .arg(url)
        .status()
        .expect("failed to execute git")
}

fn configure_cmake(path: &str) -> ExitStatus {
    let build_path = format!("{}/build", path);
    std::fs::create_dir_all(&build_path).expect("failed to create build directory");

    Command::new("cmake")
        .args(&["..", "-G", "MinGW Makefiles", "--fresh"])
        .current_dir(&build_path)
        .status()
        .expect("failed to execute cmake configuration")
}

fn build_with_cmake(path: &str) -> ExitStatus {
    let build_path = format!("{}/build", path);

    Command::new("cmake")
        .args(&["--build", ".", "--parallel"])
        .current_dir(&build_path)
        .status()
        .expect("failed to execute cmake build")
}

fn build_with_make(path: &str) -> ExitStatus {
    Command::new("make")
        .current_dir(path)
        .status()
        .expect("failed to execute make")
}

fn configure_meson(path: &str) -> ExitStatus {
    Command::new("meson")
        .arg("setup")
        .arg("build")
        .current_dir(path)
        .status()
        .expect("failed to execute meson setup")
}

fn build_with_meson(path: &str) -> ExitStatus {
    Command::new("meson")
        .arg("compile")
        .current_dir(format!("{}/build", path))
        .status()
        .expect("failed to execute meson build")
}

fn configure_cargo(path: &str) -> ExitStatus {
    Command::new("cargo")
        .arg("build")
        .current_dir(path)
        .status()
        .expect("failed to execute cargo build")
}

fn find_build_systems(path: &str) -> Vec<&'static str> {
    let mut systems = Vec::new();
    if Path::new(&format!("{}/CMakeLists.txt", path)).exists() {
        systems.push("CMake");
    }
    if Path::new(&format!("{}/Makefile", path)).exists() {
        systems.push("Make");
    }
    if Path::new(&format!("{}/meson.build", path)).exists() {
        systems.push("Meson");
    }
    if Path::new(&format!("{}/Cargo.toml", path)).exists() {
        systems.push("Cargo");
    }
    systems
}

fn choose_build_system<'a>(systems: &'a [&str]) -> Option<&'a str> {
    for (i, system) in systems.iter().enumerate() {
        println!("{}: {}", i + 1, system);
    }
    print!("Enter the number of the build system to use: ");
    io::stdout().flush().unwrap();

    let mut input = String::new();
    io::stdin().read_line(&mut input).unwrap();
    let choice = input.trim().parse::<usize>().ok()?;

    if choice > 0 && choice <= systems.len() {
        Some(systems[choice - 1])
    } 
    else {
        None
    }
}

#[tokio::main]
async fn main() {
    print!("Enter search query: ");
    io::stdout().flush().unwrap();

    let mut query = String::new();
    io::stdin().read_line(&mut query).unwrap();
    let query = query.trim();

    match search_repositories(query).await {
        Ok(repos) => {
            if let Some(repo) = choose_repository(&repos) {
                if download_repository(repo).success() {
                    let repo_name = repo.full_name.split('/').last().unwrap();
                    let build_systems = find_build_systems(repo_name);
                    if build_systems.is_empty() {
                        println!("No build systems found in the repository.");
                    } 
                    else {
                        if let Some(system) = choose_build_system(&build_systems) {
                            let status = match system {
                                "CMake" => {
                                    if configure_cmake(repo_name).success() {
                                        build_with_cmake(repo_name)
                                    } 
                                    else {
                                        eprintln!("CMake configuration failed.");
                                        return;
                                    }
                                }
                                "Make" => build_with_make(repo_name),
                                "Meson" => {
                                    if configure_meson(repo_name).success() {
                                        build_with_meson(repo_name)
                                    } 
                                    else {
                                        eprintln!("Meson configuration failed.");
                                        return;
                                    }
                                }
                                "Cargo" => {
                                    if configure_cargo(repo_name).success() {
                                        Command::new("cargo")
                                            .arg("run")
                                            .current_dir(repo_name)
                                            .status()
                                            .expect("failed to execute cargo run")
                                    } 
                                    else {
                                        eprintln!("Cargo build failed.");
                                        return;
                                    }
                                }
                                _ => {
                                    println!("Unsupported build system.");
                                    return;
                                }
                            };
                            if status.success() {
                                println!("Build completed successfully!");
                            } 
                            else {
                                println!("Build failed.");
                            }
                        } 
                        else {
                            println!("Invalid choice or no build system chosen.");
                        }
                    }
                } 
                else {
                    println!("Failed to clone repository {}", repo.full_name);
                }
            } 
            else {
                println!("Invalid choice or no repository chosen.");
            }
        }
        Err(e) => {
            eprintln!("Failed to search repositories: {}", e);
        }
    }
}