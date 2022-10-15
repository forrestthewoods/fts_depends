# fts_depends

`fts_depends` is a Rust command line tool for printing recursive binary dependencies. 

It's similar to [Dependency Walker](https://dependencywalker.com/) but much faster. Under the hood it recursively calls `dumpbin.exe`.

# Usage

Basic Usage: `fts_depends.exe path/to/bin.exe`

![](/screenshots/monkey_island_table.png?raw=true)

Tree View: `fts_depends.exe path/to/bin.exe --tree-print`

![](/screenshots/monkey_island_tree.png?raw=true)

If a dependency can't be located then it displays a clear ⚠️ Not Found ⚠️ message like this:

![](/screenshots/monkey_island_missing.png?raw=true)

# Installation

Download from [Releases](https://github.com/forrestthewoods/fts_depends/releases) or run `cargo install fts_depends`.

`fts_depends.exe` requires `dumpbin.exe`, which is typically installed with Visual Studio. If you installed Visual Studio into default location then `dumpbin.exe` with be discovered automatically. Otherwise its location can be specified via `--dumpbin path/to/dumpbin.exe`.

# Limitations

`fts_depends` is basically a wrapper around `dumpbin.exe`. It doesn't actually load any libraries. This makes it fast but less precise. Many programs and launcher scripts do horrible things to their PATH. This tool doesn't know about those runtime paths so it may report a dependency as missing when it would be found.

That said, the primary motivation behind this tool is to debug why a library will fail to load. It serves that purpose well enough.
