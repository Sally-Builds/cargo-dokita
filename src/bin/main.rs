use cargo_dokita::MyError;
use clap::{self, command, Arg, Command};

fn main() -> Result<(), MyError> {
    let commands = command!()
        .subcommand(
            Command::new("dokita")
        )
        .about("Analyzes your Rust project for common issues, adherence to best practices, potential pitfalls, and offers suggestions for improvement.").arg(
        Arg::new("project-path")
            .short('p')
            .long("project-path")
            .help("The Project Path you want to analyze")
            .default_value("./")
    ).get_matches();

    println!("project path = {}", commands.get_one::<String>("project-path").unwrap());
    let project_path = commands.get_one::<String>("project-path").unwrap();

    
    cargo_dokita::analyze_project(project_path)?;
    Ok(())
}
