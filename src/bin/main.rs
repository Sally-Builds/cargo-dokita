use cargo_dokita::MyError;
use clap::{self, command, Arg, Command};

fn main() -> Result<(), MyError> {
    let commands = command!()
        .subcommand(
            Command::new("dokita")
        )
        .about("Analyzes your Rust project for common issues, adherence to best practices, potential pitfalls, and offers suggestions for improvement.")
        .arg(
            Arg::new("project-path")
                .short('p')
                .long("project-path")
                .help("The Project Path you want to analyze")
                .default_value("./")).
        arg(
            Arg::new("FORMAT")
            .short('f')
            .long("format")
            .help("Results in either human readable or JSON format. human or json")
            .default_value("human")
        )
        .get_matches();

    println!("project path = {}", commands.get_one::<String>("project-path").unwrap());
    let project_path = commands.get_one::<String>("project-path").unwrap();
    let output_format = commands.get_one::<String>("FORMAT")
    .map(|s| s.to_ascii_lowercase())
    .filter(|s| s == "json")
    .unwrap_or_else(|| "human".to_string());

    

    
    cargo_dokita::analyze_project(project_path, &output_format)?;
    Ok(())
}
