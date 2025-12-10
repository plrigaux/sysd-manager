#![allow(clippy::uninlined_format_args)]
use std::env;

use std::path::Path;
use std::process::Command;

/* macro_rules! script_warning {
    ($($tokens: tt)*) => {
        println!("cargo::warning={}", format!($($tokens)*))
    }
}

macro_rules! script_error {
    ($($tokens: tt)*) => {
        println!("cargo::error={}", format!($($tokens)*))
    }
} */

fn main() {
    #[cfg(feature = "flatpak")]
    compile_resources(
        &["data"],
        "data/resources.gresource.xml",
        "sysd-manager-proxy.gresource",
    );
}

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
