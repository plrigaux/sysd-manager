use constcat::concat;
use log::error;
use log::info;
use std::{
    collections::HashSet,
    fs::{self, read_to_string},
    io,
    path::PathBuf,
    process::Command,
};

use crate::error::TransError;
pub mod error;

pub const MAIN_PROG: &str = "sysd-manager";
pub const PO_DIR: &str = "./po";
pub const DESKTOP_DIR: &str = "./data/applications";
pub const DESKTOP_FILE: &str = "io.github.plrigaux.sysd-manager.desktop";
pub const DESKTOP_FILE_PATH: &str = concat!(DESKTOP_DIR, "/", DESKTOP_FILE);

pub const METAINFO_DIR: &str = "./data/metainfo";
pub const METAINFO_FILE: &str = "io.github.plrigaux.sysd-manager.metainfo.xml";
pub const METAINFO_FILE_PATH: &str = concat!(METAINFO_DIR, "/", METAINFO_FILE);

pub const PACK_FILE_DIR: &str = "target/loc";

/// Making the PO Template File
/// https://www.gnu.org/software/gettext/manual/html_node/xgettext-Invocation.html
pub fn xgettext(potfiles_file_path: &str, output_pot_file: &str) {
    let mut command = Command::new("xgettext");

    for preset in glib_preset() {
        command.arg(preset);
    }

    let output = command
        .arg(format!("--files-from={potfiles_file_path}"))
        .arg(format!("--output={output_pot_file}"))
        .arg("--verbose")
        .output()
        .unwrap();

    display_output("XGETTEXT", output);
}

fn display_output(id: &str, output: std::process::Output) {
    info!("{id}: {:?}", output.status);
    info!("{id}: {}", String::from_utf8_lossy(&output.stdout));
    if !output.status.success() {
        error!("{id}: {}", String::from_utf8_lossy(&output.stderr));
    }
}

/// Creating a New PO File
/// https://www.gnu.org/software/gettext/manual/html_node/msginit-Invocation.html
pub fn msginit(input_pot_file: &str, output_file: &str, lang: &str) {
    let mut command = Command::new("msginit");

    let output = command
        .arg(format!("--input={input_pot_file}"))
        .arg(format!("--output-file={output_file}"))
        .arg(format!("--locale={lang}"))
        .output()
        .expect("command msginit ok");

    display_output("MSGINIT", output);
}

//   /usr/bin/msgmerge --update --quiet  --lang=pt_BR --previous pt_BR.po hello-rust.pot
// rm -f pt_BR.gmo && /usr/bin/msgmerge --for-msgfmt -o pt_BR.1po pt_BR.po hello-rust.pot && /usr/bin/msgfmt -c --statistics --verbose -o pt_BR.gmo pt_BR.1po && rm -f pt_BR.1po
// pt_BR.1po: 2 translated messages.

/// https://www.gnu.org/software/gettext/manual/html_node/msgmerge-Invocation.html
pub fn msgmerge(input_pot_file: &str, output_file: &str) {
    let mut command = Command::new("msgmerge");

    let output = command
        .arg("-o")
        .arg(output_file)
        .arg(output_file)
        .arg(input_pot_file)
        .arg("--verbose")
        .output()
        .unwrap();

    display_output("MSGMERGE", output);
}

pub fn generate_mo() -> Result<(), TransError> {
    for path in fs::read_dir(PO_DIR)?
        .filter_map(|r| r.ok())
        .filter_map(|dir_entry| {
            let p = dir_entry.path();
            if p.extension().is_some_and(|this_ext| this_ext == "po") {
                Some(p)
            } else {
                None
            }
        })
    {
        info!("PO file {:?} lang {:?}", path, path.file_stem());

        if let Some(po_file) = path.to_str() {
            if let Some(lang) = path.file_stem().and_then(|s| s.to_str()) {
                msgfmt(po_file, lang, MAIN_PROG)?;
            }
        }
    }

    Ok(())
}

pub fn lingas_from_files() -> Result<HashSet<String>, TransError> {
    let linguas: HashSet<_> = fs::read_dir(PO_DIR)?
        .filter_map(|r| r.ok())
        .filter_map(|dir_entry| {
            let p = dir_entry.path();
            if p.extension().is_some_and(|this_ext| this_ext == "po") {
                Some(p)
            } else {
                None
            }
        })
        .filter_map(|path| {
            path.file_stem()
                .and_then(|s| s.to_str())
                .map(|s| s.to_string())
        })
        .collect();

    Ok(linguas)
}

pub fn lingas_from_lingua_file() -> Result<HashSet<String>, TransError> {
    let mut linguas_file = PathBuf::from(PO_DIR);
    linguas_file.push("LINGUAS");

    let mut linguas = HashSet::new();

    for line in read_to_string(linguas_file)?
        .lines()
        .map(|s| s.trim())
        .filter(|s| !s.starts_with("#"))
    {
        for lang in line.split_whitespace() {
            linguas.insert(lang.to_owned());
        }
    }

    Ok(linguas)
}

pub fn generate_desktop() -> Result<(), TransError> {
    fs::create_dir_all(PACK_FILE_DIR)?;
    let out_file = format!("{}/{}", PACK_FILE_DIR, DESKTOP_FILE);

    let mut command = Command::new("msgfmt");
    let output = command
        .arg("--check")
        .arg("--statistics")
        .arg("--verbose")
        .arg("--desktop")
        .arg(format!("--template={}", DESKTOP_FILE_PATH))
        .arg("-d")
        .arg(PO_DIR)
        .arg("-o")
        .arg(out_file)
        .output()?;

    display_output("MSGFMT", output);

    Ok(())
}

pub fn generate_metainfo() -> Result<(), TransError> {
    fs::create_dir_all(PACK_FILE_DIR)?;

    let out_file = format!("{}/{}", PACK_FILE_DIR, METAINFO_FILE);

    let mut command = Command::new("msgfmt");
    let output = command
        .arg("--check")
        .arg("--statistics")
        .arg("--verbose")
        .arg("--xml")
        .arg(format!("--template={}", METAINFO_FILE_PATH))
        .arg("-d")
        .arg(PO_DIR)
        .arg("-o")
        .arg(out_file)
        .output()?;

    display_output("MSGFMT", output);

    Ok(())
}

// /usr/bin/msgfmt -c --statistics --verbose -o pt_BR.gmo pt_BR.1po && rm -f pt_BR.1po
/// Generates a binary message catalog from a textual translation description.
/// https://www.gnu.org/software/gettext/manual/html_node/msgfmt-Invocation.html
pub fn msgfmt(po_file: &str, lang: &str, domain_name: &str) -> io::Result<()> {
    let mut command = Command::new("msgfmt");

    let out_dir = format!("target/locale/{lang}/LC_MESSAGES");

    fs::create_dir_all(&out_dir)?;

    let output = command
        .arg("--check")
        .arg("--statistics")
        .arg("--verbose")
        .arg("-o")
        .arg(format!("{out_dir}/{domain_name}.mo"))
        .arg(po_file)
        .output()
        .unwrap();

    display_output("MSGFMT", output);

    Ok(())
}

fn glib_preset() -> Vec<&'static str> {
    let v = vec![
        "--from-code=UTF-8",
        "--add-comments",
        // # https://developer.gnome.org/glib/stable/glib-I18N.html
        "--keyword=_",
        "--keyword=N_",
        "--keyword=C_:1c,2",
        "--keyword=NC_:1c,2",
        "--keyword=g_dcgettext:2",
        "--keyword=g_dngettext:2,3",
        "--keyword=g_dpgettext2:2c,3",
        "--flag=N_:1:pass-c-format",
        "--flag=C_:2:pass-c-format",
        "--flag=NC_:2:pass-c-format",
        "--flag=g_dngettext:2:pass-c-format",
        "--flag=g_strdup_printf:1:c-format",
        "--flag=g_string_printf:2:c-format",
        "--flag=g_string_append_printf:2:c-format",
        "--flag=g_error_new:3:c-format",
        "--flag=g_set_error:4:c-format",
        "--flag=g_markup_printf_escaped:1:c-format",
        "--flag=g_log:3:c-format",
        "--flag=g_print:1:c-format",
        "--flag=g_printerr:1:c-format",
        "--flag=g_printf:1:c-format",
        "--flag=g_fprintf:2:c-format",
        "--flag=g_sprintf:2:c-format",
        "--flag=g_snprintf:3:c-format",
    ];
    v
}
