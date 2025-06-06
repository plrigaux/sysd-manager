use std::{fs, io, process::Command};

pub fn translating() {
    println!("test LIB");

    //POFILE identification

    //xgettext

    //msginit

    //msgfmt

    //msgmerge

    //   /usr/bin/msgmerge --update --quiet  --lang=pt_BR --previous pt_BR.po hello-rust.pot
    // rm -f pt_BR.gmo && /usr/bin/msgmerge --for-msgfmt -o pt_BR.1po pt_BR.po hello-rust.pot && /usr/bin/msgfmt -c --statistics --verbose -o pt_BR.gmo pt_BR.1po && rm -f pt_BR.1po
    // pt_BR.1po: 2 translated messages.
}

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
    println!("{id}: {:?}", output.status);
    println!("{id}: {}", String::from_utf8_lossy(&output.stdout));
    if !output.status.success() {
        eprintln!("{id}: {}", String::from_utf8_lossy(&output.stderr));
    }
}

/// Creating a New PO File
/// https://www.gnu.org/software/gettext/manual/html_node/msginit-Invocation.html
pub fn msginit(input_pot_file: &str, output_file: &str) {
    let mut command = Command::new("msginit");

    let output = command
        .arg(format!("--input={input_pot_file}"))
        .arg(format!("--output-file={output_file}"))
        //  .arg("--verbose")
        .output()
        .expect("command msginit ok");

    display_output("MSGINIT", output);
}

/// https://www.gnu.org/software/gettext/manual/html_node/msgmerge-Invocation.html
pub fn msgmerge() {
    let mut command = Command::new("msgmerge");

    for preset in glib_preset() {
        command.arg(preset);
    }

    let output = command
        .arg("--files-from=./po/POTFILES ")
        .arg("--output=./po/sysd-manager.pot")
        .arg("--verbose")
        .output()
        .unwrap();

    display_output("MSGMERGE", output);
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
