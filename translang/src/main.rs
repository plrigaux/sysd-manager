extern crate translating;

use std::io::BufRead;
use std::path::PathBuf;
use std::{fs::File, io, path::Path};

use clap::Parser;

/// A GUI interface to manage systemd units
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// Action to perform
    #[arg()]
    action: Option<String>,
}

const ACTION_GENERATE: &str = "generate";
fn main() {
    println!("Hello, world!");

    let args = Args::parse();

    match args.action {
        Some(s) if s == ACTION_GENERATE => {
            let _r = generate_missing_po();
        }
        Some(s) => println!("unknown action {:?}", s),
        None => println!("choose action: {ACTION_GENERATE}"),
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
