use clap::Parser;
use std::path::PathBuf;

#[derive(Parser, Debug)]
struct Args {
    #[arg(short, long, help = "Path to dumpbin.exe")]
    dumpbin: Option<PathBuf>,

    #[arg(help = "Binary to calculate dependencies of")]
    target: PathBuf,
}

fn main() -> anyhow::Result<()> {
    let args = Args::parse();

    // Get dumpbin path
    let dumpbin = match args.dumpbin {
        Some(path) => Ok(path),
        None => find_dumpbin(),
    }?;

    // Build command
    let mut cmd = std::process::Command::new(&dumpbin);
    cmd.arg("/DEPENDENTS");
    cmd.arg(args.target.as_os_str());

    // Invoke command and capture output
    let output = cmd.output().expect(&format!("failed to run [{:?}]", &dumpbin));

    let stdout = String::from_utf8(output.stdout)?;

    // Extract dependencies
    let start_str = "Image has the following dependencies:";
    let end_str = "Summary";
    
    let start_idx = stdout.find(start_str).expect("Failed to find start_str in stdout");
    let end_idx = stdout.find(end_str).expect("Failed to find end_str in stdout");

    let deps_str = stdout[start_idx + start_str.len()..end_idx].trim();
    let deps : Vec<&str> = deps_str.lines().map(|line| line.trim()).collect();

    println!("deps: {:?}", deps);

    Ok(())
}

fn find_dumpbin() -> anyhow::Result<PathBuf> {
    // Check path
    if let Ok(path) = which::which("dumpbin.exe") {
        return Ok(path);
    }

    // Search Visual Studio directories
    let roots = [
        "c:/Program Files/Microsoft Visual Studio",
        "c:/Program Files (x86)/Microsoft Visual Studio",
    ];

    for root in roots {
        for entry in walkdir::WalkDir::new(root) {
            if entry.is_err() {
                continue;
            }

            let entry = entry.unwrap();
            if entry.file_type().is_file() {
                if entry.file_name() == "dumpbin.exe" {
                    return Ok(entry.path().to_owned());
                }
            }
        }
    }

    // Failed to find
    anyhow::bail!("Failed to find dumpbin.exe")
}
