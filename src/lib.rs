use std::{fs, io::Error, path::PathBuf, process};


#[derive(Debug)]
pub enum MyError {
    NotRustProject,
    UnresolvableProjectPath,
}


pub struct Analyzer;

impl Analyzer {
    pub fn new() -> Analyzer {
        Analyzer {}
    }

    pub fn analyze_project (project_path: &str) -> Result<(), Error> {
        let project_path = match fs::canonicalize(project_path) {
            Ok(path) => path,
            Err(e) => {
                eprint!("Error Could not resolve project path - {:?}", e);
                process::exit(1);
            },
        };
    
        if !Analyzer::is_rust_project(project_path) {
            eprintln!("This is not a rust project");
            process::exit(1);
        }
    
        println!("All good!!!");
        Ok(())
    }

    fn is_rust_project(project_path: PathBuf) -> bool {
        if !project_path.is_dir() {
            return false
        }
    
        project_path.join("Cargo.toml").is_file()
        
    
    }
}

pub fn analyze_project (project_path: &str) -> Result<(), MyError> {
    let project_path = match fs::canonicalize(project_path) {
        Ok(path) => path,
        Err(e) => {
            eprint!("Error Could not resolve project path - {:?}", e);
            return Err(MyError::UnresolvableProjectPath)
        },
    };

    if !is_rust_project(project_path) {
        eprintln!("This is not a rust project");
        return Err(MyError::NotRustProject);
    }

    println!("All good!!!");
    Ok(())
}

fn is_rust_project(project_path: PathBuf) -> bool {
    if !project_path.is_dir() {
        return false
    }

    project_path.join("Cargo.toml").is_file()
    

}




mod tests {}