fn main() {
    compile_resources(
        &["data"],
        "data/resources/resources.gresource.xml",
        "sysd-manager.gresource",
    );
}

// BELOW CODE is COPY of glib-build-tools = "0.19.0"
// THE REASON OF THE COPY IS BECAUSE FEDORA COPR DOESN'T HAVE glib-build-tools

// Take a look at the license at the top of the repository in the LICENSE file.

use std::{env, path::Path, process::Command};

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

    println!("cargo:rerun-if-changed={gresource}");
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
        println!("cargo:rerun-if-changed={dep}");
    }
}
