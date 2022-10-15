use clap::Parser;
use std::collections::HashSet;
use std::path::{Path, PathBuf};

#[derive(Parser, Debug)]
struct Args {
    // Named arguments
    #[arg(long, help = "Path to dumpbin.exe")]
    dumpbin: Option<PathBuf>,

    #[arg(short, long, help = "Enable to show system libraries")]
    show_system: bool,

    #[arg(short = 'd', long, help = "Enable to show duplicates (tree-view only)")]
    show_dupes: bool,

    // Positional arguments
    #[arg(help = "Binary to calculate dependencies for.")]
    target: PathBuf,
}

#[derive(Default)]
struct Dependency {
    name: PathBuf,
    path: Option<PathBuf>,
    children: Vec<Box<Dependency>>,
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
    let output = cmd
        .output()
        .expect(&format!("failed to run [{:?}]", &dumpbin));
    let stdout = String::from_utf8(output.stdout)?;

    // Extract dependencies from stdout
    let deps = extract_deps(&stdout);

    let target_dir: PathBuf = args
        .target
        .parent()
        .expect(&format!(
            "Failed to get parent dir for [{:?}]",
            &args.target
        ))
        .into();

    let mut table = prettytable::Table::new();
    table.set_titles(prettytable::row![
        "Dependency",
        "Resolved Location (best guess)"
    ]);
    //table.add_row(prettytable::row!["Dep", "Loc"]);
    for dep in &deps {
        let loc_str = match which::which_in(dep, Some(&target_dir), &target_dir)
            .or_else(|_| which::which(dep))
        {
            Ok(path) => path.to_string_lossy().into_owned(),
            Err(_) => "⚠️ Not Found ⚠️".to_owned(),
        };

        table.add_row(prettytable::row![dep, loc_str.as_str()]);
        //println!("{dep} ==> {loc_str}");
    }
    table.set_format(*prettytable::format::consts::FORMAT_NO_LINESEP_WITH_TITLE);
    table.printstd();

    Ok(())
}

fn find_deps(
    dumpbin: &Path,
    target: &Path,
    target_dir: &Path,
    args: &Args,
    found_deps: &mut HashSet<PathBuf>,
) -> anyhow::Result<Dependency> {
    // Build command
    let mut cmd = std::process::Command::new(&dumpbin);
    cmd.arg("/DEPENDENTS");
    cmd.arg(args.target.as_os_str());

    // Invoke command and capture output
    let output = cmd
        .output()
        .expect(&format!("failed to run [{:?}]", &dumpbin));
    let stdout = String::from_utf8(output.stdout)?;

    // Extract dependencies from stdout
    let deps = extract_deps(&stdout);

    let mut result: Box<Dependency> = Default::default();
    result.name = target.file_name().unwrap().into();
    //result.

    anyhow::bail!("Oh no")
}

fn find_location(target: &Path, target_dir: &Path) -> which::Result<PathBuf> {
    which::which_in(target, Some(&target_dir), &target_dir).or_else(|_| which::which(target))
}

fn extract_deps(dumpbin_str: &str) -> Vec<&str> {
    // Extract dependencies
    let start_str = "Image has the following dependencies:";
    let end_str = "Summary";

    let start_idx = dumpbin_str
        .find(start_str)
        .expect("Failed to find start_str in stdout");
    let end_idx = dumpbin_str
        .find(end_str)
        .expect("Failed to find end_str in stdout");

    let deps_str = dumpbin_str[start_idx + start_str.len()..end_idx].trim();
    let deps: Vec<&str> = deps_str
        .lines()
        .map(|line| line.trim())
        .filter(|line| !line.is_empty())
        .filter(|line| *line != "Image has the following delay load dependencies:")
        .collect();

    deps
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
