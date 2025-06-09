extern crate log;
extern crate translating;

use clap::Parser;
use translating::MAIN_PROG;
use translating::PO_DIR;
use translating::error::TransError;

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
const ACTION_XGETTEXT: &str = "xgettext";
const ACTION_MO: &str = "mo";

fn main() {
    println!("Hello, world!");

    let args = Args::parse();

    let result = match args.action {
        Some(s) if s == ACTION_GENERATE => generate_missing_po(),
        Some(s) if s == ACTION_POTFILE => generate_potfiles(),
        Some(s) if s == ACTION_XGETTEXT => generate_po_template(),
        Some(s) if s == ACTION_MO => generate_mo(),
        Some(s) => {
            display_hint(Some(&s));
            Ok(())
        }
        None => {
            display_hint(None);
            Ok(())
        }
    };

    if let Err(err) = result {
        log::error!("Error {:?}", err);
    }
}

fn display_hint(unknown_action: Option<&str>) {
    if let Some(unknown_action) = unknown_action {
        println!("Unknown action {:?}", unknown_action)
    }

    let mut actions = [ACTION_GENERATE, ACTION_POTFILE, ACTION_XGETTEXT];
    actions.sort();

    let actions = actions.join(", ");

    println!("Choose following actions: {actions}");
}

fn generate_missing_po() -> Result<(), TransError> {
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

        let output_file = format!("{PO_DIR}/{lang_po}");
        let input_pot_file = format!("{PO_DIR}/sysd-manager.pot");

        if !lang_po_path.exists() {
            translating::msginit(&input_pot_file, &output_file, &lang);
        } else {
            translating::msgmerge(&input_pot_file, &format!("{PO_DIR}/{lang_po}"));
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

const POTFILES: &str = "POTFILES";
fn generate_potfiles() -> Result<(), TransError> {
    //TODO filter on gettext only
    let mut potfiles_entries = list_files("src", "rs")?;
    let mut interc = list_files("data/interfaces", "ui")?;

    potfiles_entries.append(&mut interc);
    potfiles_entries.sort();

    println!("{:#?}", potfiles_entries);

    let mut potfiles_path = PathBuf::from(PO_DIR);
    potfiles_path.push(POTFILES);

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

fn generate_po_template() -> Result<(), TransError> {
    let output_pot_file = format!("{PO_DIR}/{}.pot", MAIN_PROG);

    let potfiles_file_path = format!("{PO_DIR}/{POTFILES}");

    translating::xgettext(&potfiles_file_path, &output_pot_file);

    Ok(())
}

fn generate_mo() -> Result<(), TransError> {
    let paths = fs::read_dir(PO_DIR)?;

    for path_result in paths {
        let path = path_result?.path();
        if path.extension().is_some_and(|this_ext| this_ext == "po") {
            println!("PO file {:?} lang {:?}", path, path.file_stem());

            if let Some(po_file) = path.to_str() {
                if let Some(lang) = path.file_stem().and_then(|s| s.to_str()) {
                    translating::msgfmt(po_file, lang, MAIN_PROG)?;
                }
            }
        }
    }
    Ok(())
}
