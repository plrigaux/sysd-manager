use std::collections::HashSet;
use std::fs::OpenOptions;
use std::io::Write;

use std::io::BufRead;

use quick_xml::{
    Reader, Writer,
    events::{BytesStart, Event},
};
use translating::error::TransError;
use translating::{DESKTOP_FILE_PATH, PO_DIR};

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

    println!("cargo::rerun-if-changed={}", DESKTOP_FILE_PATH);

    translating::generate_desktop()?;

    Ok(())
}

// BELOW CODE is COPY of glib-build-tools = "0.19.0"
// THE REASON OF THE COPY IS BECAUSE FEDORA COPR DOESN'T HAVE glib-build-tools

// Take a look at the license at the top of the repository in the LICENSE file.

use std::{
    env, fs,
    path::{Path, PathBuf},
    process::Command,
};

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

    let out_dir = PathBuf::from(home_dir).join(GLIB_SCHEMAS_DIR);

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
            "Compile Schema with {GLIB_COMPILE_SCHEMAS} Failed (status {}),  directory {:?}",
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

#[derive(Debug)]
pub enum ScriptError {
    FtmError(std::fmt::Error),
    IoError(std::io::Error),
    XmlError(quick_xml::Error),
}

impl From<std::io::Error> for ScriptError {
    fn from(error: std::io::Error) -> Self {
        ScriptError::IoError(error)
    }
}

impl From<quick_xml::Error> for ScriptError {
    fn from(error: quick_xml::Error) -> Self {
        ScriptError::XmlError(error)
    }
}

fn generate_notes() -> Result<(), ScriptError> {
    const METAINFO: &str = "data/metainfo/io.github.plrigaux.sysd-manager.metainfo.xml";
    println!("cargo::rerun-if-changed={METAINFO}");

    let release_notes = match get_release_notes(METAINFO) {
        Ok(list) => list,
        Err(error) => {
            script_error!("Error parsing metainfo: {:?}", error);
            return Ok(());
        }
    };

    generate_release_notes_rs(&release_notes)?;

    #[cfg(not(feature = "flatpak"))]
    generate_changelog_md(&release_notes)?;

    Ok(())
}

fn generate_changelog_md(release_notes: &Vec<Release>) -> Result<(), ScriptError> {
    let Some(out_dir) = env::var_os("OUT_DIR") else {
        script_error!("No OUT_DIR");
        return Ok(());
    };

    const CHANGELOG: &str = "CHANGELOG.md";

    let dest_path = Path::new(&out_dir).join(CHANGELOG);
    println!("dest_path {:?}", dest_path);

    let mut w = Vec::new();

    writeln!(
        &mut w,
        r#"# Changelog
All notable changes to this project will be documented in this file.
    
The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html)."#
    )?;

    let change_type = HashSet::from([
        "Added",
        "Changed",
        "Deprecated",
        "Removed",
        "Fixed",
        "Security",
    ]);

    let mut junk_buf: Vec<u8> = Vec::new();

    for release in release_notes {
        writeln!(&mut w, "\n## [{}] - {}", release.version, release.date)?;

        let mut reader = Reader::from_str(&release.description);

        loop {
            match reader.read_event() {
                Err(e) => panic!("Error at position {}: {:?}", reader.error_position(), e),
                // exits the loop when reaching end of file
                Ok(Event::Start(e)) => match e.name().as_ref() {
                    b"p" => {
                        let content = read_to_end_into_buffer_inner(&mut reader, e, &mut junk_buf)?;
                        if change_type.contains(content.as_str()) {
                            writeln!(&mut w, "\n### {}", content)?;
                        } else {
                            writeln!(&mut w, "{}\n", content)?;
                        }
                    }
                    b"li" => {
                        let content = read_to_end_into_buffer_inner(&mut reader, e, &mut junk_buf)?;
                        writeln!(&mut w, "- {}", content)?;
                    }

                    _ => (),
                },

                Ok(Event::Eof) => break,
                _ => (),
            }
        }
    }

    fs::write(&dest_path, w)?;

    let dir = env::var("CARGO_MANIFEST_DIR").unwrap();
    let mut changelog_file = Path::new(&dir).join(CHANGELOG);
    println!("CARGO_MANIFEST_DIR {:?} ", dir);

    if !changelog_file.exists() {
        println!("File {:?} doesn't exist", changelog_file);
        let (dir, _) = dir.split_once("/target/").unwrap();
        changelog_file = Path::new(dir).join(CHANGELOG);
        if !changelog_file.exists() {
            println!("File {:?} doesn't exist", changelog_file);
            let cur_dir = env::var("PWD").unwrap();
            changelog_file = Path::new(&cur_dir).join(CHANGELOG);
        }
    }

    if !compare_files(dest_path.as_path(), changelog_file.as_path()) {
        let mut command = Command::new("cp");
        let output = command.arg("-v").arg(dest_path).arg(CHANGELOG).output()?;

        println!(
            "Copying {CHANGELOG} done {}",
            String::from_utf8_lossy(&output.stdout)
        );
    }

    Ok(())
}

use std::io::Read;
fn compare_files(file1_path: &Path, file2_path: &Path) -> bool {
    println!("compare_files {:?} with {:?}", file1_path, file2_path);
    let mut file1 = match OpenOptions::new().read(true).write(false).open(file1_path) {
        Ok(n) => n,
        Err(e) => {
            script_warning!("Could not read file {:?}! {:?}", file1_path, e);
            return false;
        }
    };

    let mut file2 = match OpenOptions::new().read(true).write(false).open(file2_path) {
        Ok(n) => n,
        Err(e) => {
            script_warning!("Could not read file {:?}! {:?}", file2_path, e);
            return false;
        }
    };

    let Ok(meta1) = file1.metadata() else {
        script_warning!("compare_files meta data error");
        return false;
    };

    let Ok(meta2) = file2.metadata() else {
        script_warning!("compare_files meta data error");
        return false;
    };

    if meta1.len() != meta2.len() {
        return false;
    }

    let mut buffer1 = Vec::new();
    let _ = file1.read_to_end(&mut buffer1);

    let mut buffer2 = Vec::new();
    let _ = file2.read_to_end(&mut buffer2);

    buffer1 == buffer2
}

fn generate_release_notes_rs(release_notes: &[Release]) -> Result<(), ScriptError> {
    let (version, description) = if let Some(first) = release_notes.first() {
        (
            format!("Some(\"{}\")", first.version),
            format!("Some(\"{}\")", first.description),
        )
    } else {
        ("None".to_owned(), "None".to_owned())
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
        "pub const RELEASE_NOTES : Option<&str> = {};",
        description
    )?;

    fs::write(&dest_path, w)?;

    Ok(())
}

#[derive(Debug, Default, Clone)]
struct Release {
    version: String,
    date: String,
    description: String,
}

fn get_release_notes(metainfo: &str) -> Result<Vec<Release>, quick_xml::Error> {
    let mut reader = Reader::from_file(metainfo)?;
    reader.config_mut().trim_text(true);

    let mut buf = Vec::new();
    let mut junk_buf: Vec<u8> = Vec::new();

    let mut release = Release::default();
    let mut in_release = false;
    let mut release_notes = Vec::new();
    loop {
        match reader.read_event_into(&mut buf) {
            Err(e) => panic!("Error at position {}: {:?}", reader.error_position(), e),
            // exits the loop when reaching end of file
            Ok(Event::Start(e)) => match e.name().as_ref() {
                b"release" => {
                    in_release = true;
                    release = Release::default();

                    for attr in e.attributes() {
                        let attr = attr.unwrap();

                        match attr.key.local_name().as_ref() {
                            b"version" => {
                                release.version = String::from_utf8_lossy(&attr.value).to_string()
                            }
                            b"date" => {
                                release.date = String::from_utf8_lossy(&attr.value).to_string()
                            }
                            _ => (),
                        }
                    }
                }
                b"description" => {
                    if !in_release {
                        continue;
                    }

                    let content =
                        read_to_end_into_buffer_inner(&mut reader, e, &mut junk_buf).unwrap();

                    release.description = content;

                    println!("Release: {:?}", release);
                }

                _ => (),
            },
            Ok(Event::End(e)) if e.name().as_ref() == b"release" => {
                release_notes.push(release.clone());
                in_release = false
            }
            Ok(Event::Eof) => break,
            _ => (),
        }
    }
    Ok(release_notes)
}

fn read_to_end_into_buffer_inner<R: BufRead>(
    reader: &mut Reader<R>,
    start_tag: BytesStart,
    junk_buf: &mut Vec<u8>,
) -> Result<String, quick_xml::Error> {
    let mut depth = 0;
    let mut output_buf: Vec<u8> = Vec::new();
    let mut w = Writer::new(&mut output_buf);
    let tag_name = start_tag.name();

    loop {
        junk_buf.clear();
        let event = reader.read_event_into(junk_buf)?;
        match event {
            Event::Start(ref e) => {
                if e.name() == tag_name {
                    depth += 1
                }
                w.write_event(event.borrow())?;
            }
            Event::End(ref e) => {
                if e.name() == tag_name {
                    if depth == 0 {
                        break;
                    }
                    depth -= 1;
                } else {
                    w.write_event(event.borrow())?;
                }
            }
            Event::Text(ref _e) => {
                w.write_event(event)?;
            }
            Event::Eof => {
                panic!("oh no")
            }
            _ => {}
        }
    }

    let s = String::from_utf8_lossy(&output_buf);
    Ok(s.to_string())
}
