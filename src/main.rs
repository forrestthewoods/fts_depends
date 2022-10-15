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
    println!("dumpbin: {:?}", dumpbin);


    // Get dumpbin.exe dir
    let dumpbin_dir = dumpbin.parent().expect("Failed to get parent dir");
    println!("dumpbin_dir: {:?}", dumpbin_dir);

    let filename = dumpbin.file_name().expect("Failed to get filename");
    println!("dumpbin_filename: {:?}", filename);

    let target_str = args.target.to_string_lossy();

    let cmd = std::process::Command::new(&filename)
        .args(["/DEPENDENTS", &target_str])
        .current_dir(dumpbin_dir);


    let output = cmd.output()
        .expect("failed invoke dumpbin.exe");

    println!("{:?}", output);

    // Build command
    /*
    let mut cmd = OsString::new();
    //cmd.push("\"");
    cmd.push(dumpbin.as_os_str());
    cmd.push(" /?");
    //cmd.push("\" /DEPENDENTS ");
    //cmd.push(args.target.as_os_str());
    */

    //println!("Invoking command: {:?}", &cmd);

    // Invoke command and capture output
    /*
    let out = subprocess::Exec::cmd(cmd)
        .stdout(subprocess::Redirection::Pipe)
        .capture()?
        .stdout_str();
    */

    /*
    std::env::set_current_dir("C:\\Program Files\\Microsoft Visual Studio\\2022\\Community\\VC\\Tools\\MSVC\\14.32.31326\\bin\\Hostx64\\x64")?;
    let mut p = subprocess::Popen::create(
        &["dumpbin.exe", "c:/stuff/path/cvdump.exe"],
        subprocess::PopenConfig {
            stdout: subprocess::Redirection::Pipe,
            ..Default::default()
        },
    )?;

    // Obtain the output from the standard streams.
    let (out, err) = p.communicate(None)?;

    if let Some(exit_status) = p.poll() {
        // the process has finished
    } else {
        // it is still running, terminate it
        p.terminate()?;
    }

    println!("cout: {:?}", out);
    println!("cerr: {:?}", err);
    */

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
