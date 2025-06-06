extern crate translating;

use clap::Parser;
//use std::error::Error;
use std::fs;
use std::io::BufRead;
use std::io::Write;
use std::path::PathBuf;
use std::{fs::File, io, path::Path};

/// A GUI interface to manage systemd units
#[derive(Parser, Debug)]
#[command()]
struct Args {
    /// Action to perform
    #[arg()]
    action: Option<String>,
}

const ACTION_GENERATE: &str = "generate";
const ACTION_POTFILE: &str = "potfiles";

fn main() {
    println!("Hello, world!");

    let args = Args::parse();

    match args.action {
        Some(s) if s == ACTION_GENERATE => {
            let _r = generate_missing_po();
        }
        Some(s) if s == ACTION_POTFILE => {
            let _r = generate_pot_files();
        }
        Some(s) => println!("unknown action {:?}", s),
        None => println!("choose action: {}, {}", ACTION_GENERATE, ACTION_POTFILE),
    }
}

const PO_DIR: &str = "./po";

fn generate_missing_po() -> io::Result<()> {
    //open file LINGUA

    let po_dir = PathBuf::from(PO_DIR);

    let mut linguas_dir = po_dir.clone();
    linguas_dir.push("LINGUAS");

    let lines = read_lines(linguas_dir)?;

    let mut linguas = Vec::new();
    for line in lines {
        let line = line.expect("read line should be ok");

        let line = line.trim();

        if !line.starts_with('#') {
            linguas.push(line.to_owned());
        }
    }
    //parse

    //write file

    println!("{:?}", linguas);

    for lang in linguas {
        let mut lang_po_path = po_dir.clone();
        let lang_po = format!("{lang}.po");

        lang_po_path.push(&lang_po);

        println!(
            "path {} exists {}",
            lang_po_path.display(),
            lang_po_path.exists()
        );

        if !lang_po_path.exists() {
            let mut input_pot_file = po_dir.clone();
            input_pot_file.push("sysd-manager.pot");

            let input_pot_file = input_pot_file
                .into_os_string()
                .into_string()
                .expect("get path to string");

            translating::msginit(&input_pot_file, &format!("{PO_DIR}/{lang_po}"));
        }
    }

    Ok(())
}

// The output is wrapped in a Result to allow matching on errors.
// Returns an Iterator to the Reader of the lines of the file.
fn read_lines<P>(filename: P) -> io::Result<io::Lines<io::BufReader<File>>>
where
    P: AsRef<Path>,
{
    let file = File::open(filename)?;
    Ok(io::BufReader::new(file).lines())
}

fn generate_pot_files() -> Result<(), Box<dyn std::error::Error>> {
    //TODO filter on gettext only
    let mut potfiles_entries = list_files("src", "rs")?;
    let mut interc = list_files("data/interfaces", "ui")?;

    // The order in which `read_dir` returns entries is not guaranteed. If reproducible
    // ordering is required the entries should be explicitly sorted.

    potfiles_entries.append(&mut interc);
    potfiles_entries.sort();

    println!("{:#?}", potfiles_entries);

    let mut potfiles_path = PathBuf::from(PO_DIR);
    potfiles_path.push("POTFILES");

    let mut potfiles_file = File::create(potfiles_path)?;

    writeln!(potfiles_file, "#File generated")?;

    for file_path in potfiles_entries {
        let file = file_path
            .into_os_string()
            .into_string()
            .expect("get path to string");

        writeln!(potfiles_file, "{}", file)?;
    }

    Ok(())
}

fn list_files<T: Into<PathBuf>>(path: T, ext: &str) -> io::Result<Vec<PathBuf>> {
    let mut files = Vec::new();
    let path = path.into();
    list_files_recurse(&mut files, path, ext)?;
    Ok(files)
}

fn list_files_recurse(files: &mut Vec<PathBuf>, path: PathBuf, ext: &str) -> io::Result<()> {
    if path.is_dir() {
        let paths = fs::read_dir(&path)?;
        for path_result in paths {
            let full_path = path_result?.path();
            list_files_recurse(files, full_path, ext)?;
        }
    } else if path.extension().is_some_and(|this_ext| this_ext == ext) {
        files.push(path);
    }
    Ok(())
}
