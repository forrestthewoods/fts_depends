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

#[derive(thiserror::Error, Debug)]
enum DependsError {
    #[error("Skipped system file")]
    SkippedSystem,

    #[error("⚠️ File not found ⚠️")]
    NotFound,
}

fn printer(table: &mut prettytable::Table, dep: &Dependency, depth: i32) {
    let loc_str = match &dep.path {
        Some(path) => path.to_string_lossy().into_owned(),
        None => "⚠️ Not Found ⚠️".to_owned(),
    };
    table.add_row(prettytable::row![
        &dep.name.to_string_lossy(),
        loc_str.as_str()
    ]);

    for child in &dep.children {
        printer(table, child, depth + 1);
    }
}

fn main() -> anyhow::Result<()> {
    let args = Args::parse();

    // Get dumpbin path
    let dumpbin: PathBuf = match &args.dumpbin {
        Some(path) => Ok(path.clone()),
        None => find_dumpbin(),
    }?;

    let target_dir: PathBuf = args
        .target
        .parent()
        .expect(&format!(
            "Failed to get parent dir for [{:?}]",
            &args.target
        ))
        .into();

    let mut visited: HashSet<PathBuf> = Default::default();
    visited.insert(args.target.file_name().unwrap().into());

    let deps = find_deps(&dumpbin, &args.target, &target_dir, &args, &mut visited);

    let mut table = prettytable::Table::new();
    table.set_titles(prettytable::row![
        "Dependency",
        "Resolved Location (best guess)"
    ]);
    //table.add_row(prettytable::row!["Dep", "Loc"]);
    for dep in &deps {
        printer(&mut table, dep, 0);
    }
    table.set_format(*prettytable::format::consts::FORMAT_NO_LINESEP_WITH_TITLE);
    table.printstd();

    /*
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
    */

    Ok(())
}

// algorithm
// find dumpbin
// run dumpbin on initial target
// get list of dependency filenames
// cull system and dupes
// repeat for child

fn find_deps(
    dumpbin: &Path,
    target: &Path,
    target_dir: &Path,
    args: &Args,
    visited: &mut HashSet<PathBuf>,
) -> anyhow::Result<Box<Dependency>> {
    let target_loc = find_location(target, target_dir).or(Err(DependsError::NotFound))?;

    if !args.show_system {
        let lossy_loc = target_loc.to_string_lossy().to_lowercase();
        if lossy_loc.contains("windows\\system32") {
            println!("Skipping system lib: {lossy_loc}");
            Err(DependsError::SkippedSystem)?;
        }
    }

    // Build command
    let mut cmd = std::process::Command::new(&dumpbin);
    cmd.arg("/DEPENDENTS");
    cmd.arg(target_loc.as_os_str());

    // Invoke command and capture output
    let output = cmd
        .output()
        .expect(&format!("failed to run [{:?}]", &dumpbin));
    let stdout = String::from_utf8(output.stdout)?;

    // Extract dependencies from stdout
    let deps = extract_deps(&stdout)?;

    let mut result: Box<Dependency> = Default::default();
    result.name = target_loc.file_name().unwrap().into();
    result.path = Some(target_loc);

    let target_loc_dir: PathBuf = target
        .parent()
        .expect(&format!(
            "Failed to get parent dir for [{:?}]",
            &args.target
        ))
        .into();

    for dep in deps {
        let inserted = visited.insert(dep.into());
        if !inserted {
            continue;
        }

        match find_deps(dumpbin, &PathBuf::from(dep), &target_loc_dir, args, visited) {
            Ok(recursive_dep) => result.children.push(recursive_dep),
            Err(err) => {
                match err.downcast_ref::<DependsError>() {
                    Some(DependsError::SkippedSystem) => continue,
                    Some(DependsError::NotFound) | None => (),
                }
                result.children.push(Box::new(Dependency {
                    name: dep.into(),
                    path: None,
                    children: Default::default(),
                }))
            }
        };
    }

    Ok(result)
}

fn find_location(target: &Path, target_dir: &Path) -> which::Result<PathBuf> {
    let cwd = "";
    which::which_in(target, Some(target_dir), cwd).or_else(|_| which::which(target))
}

fn extract_deps(dumpbin_str: &str) -> anyhow::Result<Vec<&str>> {
    // Extract dependencies
    let start_str = "Image has the following dependencies:";
    let end_str = "Summary";

    let start_idx = dumpbin_str
        .find(start_str)
        .ok_or_else(|| anyhow::anyhow!("no dependencies"))?;
    let end_idx = dumpbin_str.find(end_str).expect("failed to find summary");

    let deps_str = dumpbin_str[start_idx + start_str.len()..end_idx].trim();
    let deps: Vec<&str> = deps_str
        .lines()
        .map(|line| line.trim())
        .filter(|line| !line.is_empty())
        .filter(|line| *line != "Image has the following delay load dependencies:")
        .collect();

    Ok(deps)
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
