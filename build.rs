//#![allow(clippy::uninlined_format_args)]
use std::{env, fs, io::Write, path::Path, process::Command};
use tool::{Release, errors::ToolError, find_change_log_file};
use translating::PO_DIR;
use translating::error::TransError;

macro_rules! script_warning {
    ($($tokens: tt)*) => {
        println!("cargo::warning={}", format!($($tokens)*))
    }
}

macro_rules! script_error {
    ($($tokens: tt)*) => {
        println!("cargo::error={}", format!($($tokens)*))
    }
}

fn main() {
    compile_resources(
        &["data"],
        "data/resources/resources.gresource.xml",
        "sysd-manager.gresource",
    );

    #[cfg(debug_assertions)]
    compile_schema();

    if let Err(error) = generate_notes() {
        script_error!("Generate release notes error : {:?}", error);
    }

    if let Err(error) = generate_mo() {
        script_error!("Generate release mo files error : {:?}", error);
    }
}

pub fn check_linguas() -> Result<(), TransError> {
    let set1 = translating::lingas_from_files()?;
    let set2 = translating::lingas_from_lingua_file()?;

    let mut vec: Vec<_> = set1.iter().filter(move |s| !set2.contains(*s)).collect();
    vec.sort();

    if !vec.is_empty() {
        script_warning!("Those languages {:?} not in LINGUAS file!", vec);
    }

    Ok(())
}

fn generate_mo() -> Result<(), TransError> {
    println!("generate_mo");
    println!("cargo::rerun-if-changed={PO_DIR}");

    check_linguas()?;

    translating::generate_mo()?;

    Ok(())
}

// BELOW CODE is COPY of glib-build-tools = "0.19.0"
// THE REASON OF THE COPY IS BECAUSE FEDORA COPR DOESN'T HAVE glib-build-tools

// Take a look at the license at the top of the repository in the LICENSE file.

// rustdoc-stripper-ignore-next
/// Call to run `glib-compile-resources` to generate compiled gresources to embed
/// in binary with [`gio::resources_register_include`]. `target` is relative to `OUT_DIR`.
///
/// ```no_run
/// glib_build_tools::compile_resources(
///     &["resources"],
///     "resources/resources.gresource.xml",
///     "compiled.gresource",
/// );
/// ```
pub fn compile_resources<P: AsRef<Path>>(source_dirs: &[P], gresource: &str, target: &str) {
    let out_dir = env::var("OUT_DIR").unwrap();
    let out_dir = Path::new(&out_dir);

    let mut command = Command::new("glib-compile-resources");

    for source_dir in source_dirs {
        command.arg("--sourcedir").arg(source_dir.as_ref());
    }

    let output = command
        .arg("--target")
        .arg(out_dir.join(target))
        .arg(gresource)
        .output()
        .unwrap();

    let path = env::current_dir().expect("env::current_dir() FAIL");
    println!("The current directory is {}", path.display());

    println!("CMD Output: {:#?}", output);

    assert!(
        output.status.success(),
        "glib-compile-resources failed with exit status {} and stderr:\n{}",
        output.status,
        String::from_utf8_lossy(&output.stderr)
    );

    println!("cargo::rerun-if-changed={gresource}");
    let mut command = Command::new("glib-compile-resources");

    for source_dir in source_dirs {
        command.arg("--sourcedir").arg(source_dir.as_ref());
    }

    let output = command
        .arg("--generate-dependencies")
        .arg(gresource)
        .output()
        .unwrap()
        .stdout;
    let output = String::from_utf8(output).unwrap();

    for dep in output.split_whitespace() {
        println!("cargo::rerun-if-changed={dep}");
    }
}

#[cfg(debug_assertions)]
fn compile_schema() {
    const GLIB_SCHEMAS_DIR: &str = ".local/share/glib-2.0/schemas/";
    const GLIB_SCHEMAS_FILE: &str = "data/schemas/io.github.plrigaux.sysd-manager.gschema.xml";

    let path = Path::new(GLIB_SCHEMAS_FILE);
    println!("Path {:?}", path);
    let schema_file = match fs::canonicalize(path) {
        Ok(s) => s,
        Err(e) => {
            println!("Error: {:?}", e);
            return;
        }
    };

    let home_dir = env::var("HOME").unwrap();

    let out_dir = std::path::PathBuf::from(home_dir).join(GLIB_SCHEMAS_DIR);

    println!("print out_dir {:?}", out_dir);

    println!("cargo::rerun-if-changed={GLIB_SCHEMAS_FILE}");
    let mut command = Command::new("install");
    let output = command
        .arg("-v")
        .arg("-D")
        .arg(schema_file)
        .arg("-t")
        .arg(&out_dir)
        .output()
        .unwrap();

    println!("Install Schema");
    println!(
        "Install Schema stdout {}",
        String::from_utf8_lossy(&output.stdout)
    );
    println!(
        "Install Schema stderr {}",
        String::from_utf8_lossy(&output.stderr)
    );
    println!("Install Schema status {}", output.status);

    const GLIB_COMPILE_SCHEMAS: &str = "glib-compile-schemas";
    let mut command = Command::new(GLIB_COMPILE_SCHEMAS);
    let output = command.arg("--strict").arg(&out_dir).output().unwrap();

    if output.status.success() {
        println!("Compile Schema Succeed on {:?}", out_dir);
    } else {
        script_error!(
            "Compile Schema with program {GLIB_COMPILE_SCHEMAS:?} Failed (status {}),  directory {:?}",
            output.status,
            out_dir
        );

        script_warning!(
            "Compile Schema stdout {}",
            String::from_utf8_lossy(&output.stdout)
        );

        script_error!(
            "Compile Schema stderr {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
}

fn generate_notes() -> Result<(), ToolError> {
    // const METAINFO: &str = "data/metainfo/io.github.plrigaux.sysd-manager.metainfo.xml";
    const CHANGELOG: &str = "CHANGELOG.md";
    println!("cargo::rerun-if-changed={CHANGELOG}");

    let dir = env::current_dir()?;

    let changelog_path = tool::find_change_log_file(&dir, CHANGELOG)?;

    let releases = tool::extract_changelog(&changelog_path)?;

    let file_path = find_change_log_file(
        &dir,
        "data/metainfo/io.github.plrigaux.sysd-manager.releases.xml",
    )?;

    tool::write_releases_to_xml(&file_path, &releases)?;
    // let release_notes = match get_release_notes(METAINFO) {
    //     Ok(list) => list,
    //     Err(error) => {
    //         script_error!("Error parsing metainfo: {:?}", error);
    //         return Ok(());
    //     }
    // };

    generate_release_notes_rs(&releases)?;

    Ok(())
}

fn generate_release_notes_rs(release_notes: &[Release]) -> Result<(), ToolError> {
    let (version, date, description) = if let Some(first) = release_notes.first() {
        let notes = tool::get_about_what_change(first)?;
        (
            format!("Some(r###\"{}\"###)", first.version),
            format!("Some(r###\"{}\"###)", first.date),
            format!("Some(r###\"{}\"###)", notes),
        )
    } else {
        ("None".to_owned(), "None".to_owned(), "None".to_owned())
    };

    let Some(out_dir) = env::var_os("OUT_DIR") else {
        script_error!("No OUT_DIR");
        return Ok(());
    };

    let dest_path = Path::new(&out_dir).join("release_notes.rs");
    println!("dest_path {:?}", dest_path);

    let mut w = Vec::new();
    writeln!(
        &mut w,
        "pub const RELEASE_NOTES_VERSION : Option<&str> = {};",
        version
    )?;

    writeln!(
        &mut w,
        "pub const RELEASE_NOTES_DATE : Option<&str> = {};",
        date
    )?;

    writeln!(
        &mut w,
        "pub const RELEASE_NOTES : Option<&str> = {};",
        description
    )?;

    fs::write(&dest_path, w)?;

    Ok(())
}
