use clap::Parser;
use ptree::TreeBuilder;
use std::collections::HashSet;
use std::path::{Path, PathBuf};

#[derive(Parser, Debug)]
struct Args {
    // Named arguments
    #[arg(long, help = "Path to dumpbin.exe")]
    dumpbin: Option<PathBuf>,

    #[arg(short, long, help = "Enable to show system libraries")]
    show_system: bool,

    #[arg(short = 'd', long, help = "Enable to print dependencies as a tree")]
    tree_print: bool,

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

    let deps = find_deps(&dumpbin, &args.target, &target_dir, &args, &mut visited)?;

    print(&deps, &args);

    Ok(())
}

fn print(root: &Dependency, args: &Args) {
    if args.tree_print {
        print_tree(root);
    } else {
        print_table(root);
    }
}

fn print_table(root: &Dependency) {
    let mut table = prettytable::Table::new();
    table.set_titles(prettytable::row![
        "Dependency",
        "Resolved Location (best guess)"
    ]);

    print_table_dep(&mut table, root);
    table.set_format(*prettytable::format::consts::FORMAT_NO_LINESEP_WITH_TITLE);
    table.printstd();
}

fn print_table_dep(table: &mut prettytable::Table, dep: &Dependency) {
    let loc_str = match &dep.path {
        Some(path) => path.to_string_lossy().into_owned(),
        None => "⚠️ Not Found ⚠️".to_owned(),
    };
    table.add_row(prettytable::row![
        &dep.name.to_string_lossy(),
        loc_str.as_str()
    ]);

    for child in &dep.children {
        print_table_dep(table, child);
    }
}

fn print_tree(root: &Dependency) {
    let mut tree = TreeBuilder::new(root.name.to_string_lossy().to_string());
    for child in &root.children {
        add_tree_child(&mut tree, child);
    }
    ptree::print_tree(&tree.build()).unwrap();
}

fn add_tree_child(tree: &mut ptree::TreeBuilder, dep: &Dependency) {
    tree.begin_child(dep.name.to_string_lossy().to_string());
    for child in &dep.children {
        add_tree_child(tree, child);
    }
    tree.end_child();
}

fn find_deps(
    dumpbin: &Path,
    target: &Path,
    target_dir: &Path,
    args: &Args,
    visited: &mut HashSet<PathBuf>,
) -> anyhow::Result<Box<Dependency>> {
    // Skip libraries that are known system libs
    if !args.show_system {
        // for some reason target.starts_with("foo") returns false
        let lossy_target = target.to_string_lossy();
        if lossy_target.starts_with("api-ms-win") || lossy_target.starts_with("ext-ms-win") {
            Err(DependsError::SkippedSystem)?;
        }
    }

    // Figure out where target actually lives
    let target_loc = find_location(target, target_dir).or(Err(DependsError::NotFound))?;

    // Skip libs that live in system directories
    if !args.show_system {
        let lossy_loc = target_loc.to_string_lossy().to_lowercase();
        if lossy_loc.contains("windows\\system32") || lossy_loc.contains("\\Windows Kits\\") {
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
    result.path = Some(target_loc.clone());

    let target_loc_dir: PathBuf = target_loc
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
    let mut deps: Vec<&str> = Default::default();

    // Extract dependencies
    let deps_str = "Image has the following dependencies:";
    let deps_idx = dumpbin_str.find(deps_str);
    if let Some(deps_idx) = deps_idx {
        // Extract deps
        let substr = dumpbin_str[deps_idx + deps_str.len()..].trim_start();
        deps.extend(
            substr
                .lines()
                .take_while(|line| !line.is_empty())
                .map(|line| line.trim()),
        );
    }

    // Extract delay load dependencies
    let deps_str = "Image has the following delay load dependencies:";
    let deps_idx = dumpbin_str.find(deps_str);
    if let Some(deps_idx) = deps_idx {
        // Extract delay deps
        let substr = dumpbin_str[deps_idx + deps_str.len()..].trim_start();
        deps.extend(
            substr
                .lines()
                .take_while(|line| !line.is_empty())
                .map(|line| line.trim()),
        );
    }

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
