extern crate log;
extern crate translating;

use clap::Command;
use clap::Parser;
use clap::Subcommand;
use log::error;
use log::{info, warn};
use translating::DESKTOP_FILE_PATH;
use translating::MAIN_PROG;
use translating::METAINFO_FILE_PATH;
use translating::PO_DIR;
use translating::error::TransError;

use std::fs;
use std::io::Write;
use std::path::PathBuf;
use std::{fs::File, io};

/// A GUI interface to manage systemd units
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Extract translation text form code and generate a Portable Object Template (pot) file
    Extract {
        // Donot regenerate the Potfile list
        #[arg(short, long)]
        no_gen: bool,
    },

    /// Generate the POTFILES. i.e. the file containign the list of source files used for the translation text extraction
    Potfile,

    /// Update po files
    Po {
        /// The po file language. Pass \"all\" if you want all of them
        #[arg(short, long)]
        lang: Vec<String>,
    },

    /// Generate po files
    Newpo {
        /// The po file language
        #[arg(short, long)]
        lang: Vec<String>,
    },

    /// Extract translation text and Generate missing po files or update them in one command
    Expo {
        /// The po file language. Pass \"all\" if you want all of them
        #[arg(short, long)]
        lang: Vec<String>,

        // Donot regenerate the Potfile list
        #[arg(short, long)]
        no_gen: bool,
    },

    /// Generate all Machine Object files
    Mo,

    /// Generate desktop file
    Desktop,

    /// Generate package file
    Packfiles,
}

fn main() {
    // Force log level
    if let Err(err) = env_logger::builder()
        .target(env_logger::Target::Stdout)
        .filter_level(log::LevelFilter::Debug)
        .try_init()
    {
        eprintln!("Logger error {err:?}")
    }

    info!("Tanslation tool!");

    let args = Args::parse();

    let result = execute_command(args);

    if let Err(err) = result {
        error!("Error {err:?}");
    }
}

fn execute_command(args: Args) -> Result<(), TransError> {
    match &args.command {
        Some(Commands::Mo) => translating::generate_mo(),
        Some(Commands::Desktop) => translating::generate_desktop(),
        Some(Commands::Po { lang }) => update_po_file(lang),
        Some(Commands::Newpo { lang }) => generate_po_file(lang),
        Some(Commands::Expo { lang, no_gen }) => {
            if *no_gen {
                generate_potfiles()?
            }
            extract_and_generate_po_template()?;

            update_po_file(lang)
        }
        Some(Commands::Extract { no_gen }) => {
            if *no_gen {
                generate_potfiles()?
            }
            extract_and_generate_po_template()
        }
        Some(Commands::Potfile) => generate_potfiles(),
        Some(Commands::Packfiles) => generate_pack(),
        None => {
            println!("Unknown command. Use \"help\" to know what is available \n");

            let mut cmd = Command::new("transtools");
            let _ = cmd.print_long_help();
            Ok(())
        }
    }
}

pub fn check_linguas() -> Result<(), TransError> {
    let set1 = translating::lingas_from_files()?;
    let set2 = translating::lingas_from_lingua_file()?;

    let mut vec: Vec<_> = set1.iter().filter(move |s| !set2.contains(*s)).collect();
    vec.sort();

    if !vec.is_empty() {
        warn!("Those languages {vec:?} not in LINGUAS file!");
    }

    Ok(())
}

fn generate_pack() -> Result<(), TransError> {
    println!("generate_mo");

    check_linguas()?;

    translating::generate_desktop()?;
    translating::generate_metainfo()?;

    Ok(())
}

fn generate_po_file(linguas: &[String]) -> Result<(), TransError> {
    let po_dir = PathBuf::from(PO_DIR);

    if !po_dir.exists() {
        return Err(TransError::PathNotExist(PO_DIR.to_owned()));
    }

    for lang in linguas {
        let mut lang_po_path = po_dir.clone();
        let lang_po = format!("{lang}.po");

        lang_po_path.push(&lang_po);

        info!(
            "path {} exists {}",
            lang_po_path.display(),
            lang_po_path.exists()
        );

        let output_file = format!("{PO_DIR}/{lang_po}");
        let input_pot_file = format!("{PO_DIR}/sysd-manager.pot");

        if !lang_po_path.exists() {
            translating::msginit(&input_pot_file, &output_file, lang);
        } else {
            info!("{output_file} already exist. Do nothing.");
        }
    }

    Ok(())
}

fn update_po_file(linguas: &[String]) -> Result<(), TransError> {
    let po_dir = PathBuf::from(PO_DIR);

    if !po_dir.exists() {
        return Err(TransError::PathNotExist(PO_DIR.to_owned()));
    }

    if !po_dir.is_dir() {
        return Err(TransError::PathNotDIR(PO_DIR.to_owned()));
    }

    let all = linguas.iter().any(|s| s.eq_ignore_ascii_case("all"));

    let mut po_files: Vec<_> = fs::read_dir(po_dir)?
        .filter_map(|r| r.ok())
        .map(|res| res.path())
        .filter(|p| {
            if let Some(ext) = p.extension() {
                ext == "po"
            } else {
                false
            }
        })
        .collect();

    /*        .map(|p| let a = p.clone(); (p.clone(), a.file_stem()))
    .filter(|f| f.1.is_some())
    .map(|(a, b)| (a, b.unwrap().to_str()))
    .filter(|(a, b)| b.is_some())
    .map(|(a, b)| (a, b.unwrap())) */

    let mut lang_files: Vec<(PathBuf, String)> = Vec::new();
    for p in po_files.drain(..) {
        if let Some(f) = p.file_stem()
            && let Some(s) = f.to_str()
        {
            lang_files.push((p.clone(), s.to_owned()));
        }
    }

    let limited: Vec<_> = lang_files
        .iter()
        .filter(|(_, b)| {
            if all {
                true
            } else {
                linguas.iter().any(|s| **s == *b)
            }
        })
        .collect();

    if limited.is_empty() {
        warn!("Need to provide one valid language or \"all\" to perform this action");

        let mut valid: Vec<_> = lang_files.iter().map(|(_, b)| b.clone()).collect();
        valid.sort();

        warn!("Valid languages currently are: {}", valid.join(", "));
        return Err(TransError::LanguageNotSet);
    };

    let input_pot_file = format!("{PO_DIR}/sysd-manager.pot");

    for (path, _lang) in limited {
        translating::msgmerge(&input_pot_file, &path.to_string_lossy())?;
    }

    Ok(())
}

const POTFILES: &str = "POTFILES";
fn generate_potfiles() -> Result<(), TransError> {
    //TODO filter on gettext only
    let mut potfiles_entries = list_files("src", "rs")?;
    let mut interc = list_files("data/interfaces", "ui")?;
    let desktop = PathBuf::from(DESKTOP_FILE_PATH);
    let metainfo = PathBuf::from(METAINFO_FILE_PATH);

    potfiles_entries.append(&mut interc);
    potfiles_entries.push(desktop);
    potfiles_entries.push(metainfo);
    potfiles_entries.sort();

    println!("{potfiles_entries:#?}");

    let mut potfiles_path = PathBuf::from(PO_DIR);
    potfiles_path.push(POTFILES);

    let mut potfiles_file = File::create(potfiles_path)?;

    writeln!(potfiles_file, "#File generated")?;

    for file_path in potfiles_entries {
        let file = file_path
            .into_os_string()
            .into_string()
            .expect("get path to string");

        writeln!(potfiles_file, "{file}")?;
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

fn extract_and_generate_po_template() -> Result<(), TransError> {
    let output_pot_file = format!("{PO_DIR}/{MAIN_PROG}.pot");

    let potfiles_file_path = format!("{PO_DIR}/{POTFILES}");

    translating::xgettext(&potfiles_file_path, &output_pot_file);

    Ok(())
}
