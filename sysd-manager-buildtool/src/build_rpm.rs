use rpm::{Dependency, FileOptions, Package, PackageBuilder};
use std::{
    env,
    path::{Path, PathBuf},
};
use toml::Table;
use tool_lib::errors::ToolError;
use tracing::{debug, info};

pub fn build_rpm() -> Result<(), ToolError> {
    let version = env!("CARGO_PKG_VERSION");
    let name = env!("CARGO_PKG_NAME");
    debug!("{name} version {version}");
    let cur_path = env::current_dir()?;

    info!("current path {}", cur_path.display());

    let p = if Some(name) == cur_path.file_name().and_then(|s| s.to_str()) {
        cur_path.parent()
    } else {
        Some(cur_path.as_path())
    };

    let Some(path) = p else {
        return Err(ToolError::CargoNotFound);
    };

    env::set_current_dir(path)?;

    let path = path.join("Cargo.toml");

    debug!("read cargo file {}", path.display());
    let config_str = std::fs::read_to_string(path)?;

    let parsed = config_str.parse::<Table>()?;

    const PACKAGE: &str = "package";
    let Some(version) = parsed
        .get(PACKAGE)
        .and_then(|v| v.as_table())
        .and_then(|v| v.get("version"))
        .and_then(|v| v.as_str())
    else {
        return Err(ToolError::Custom("No version"));
    };

    const WORKSPACE: &str = "workspace";
    let Some(license) = parsed
        .get(WORKSPACE)
        .and_then(|v| v.as_table())
        .and_then(|v| v.get(PACKAGE))
        .and_then(|v| v.as_table())
        .and_then(|v| v.get("license"))
        .and_then(|v| v.as_str())
    else {
        return Err(ToolError::Custom("No licence"));
    };

    let Some(repository) = parsed
        .get(WORKSPACE)
        .and_then(|v| v.as_table())
        .and_then(|v| v.get(PACKAGE))
        .and_then(|v| v.as_table())
        .and_then(|v| v.get("repository"))
        .and_then(|v| v.as_str())
    else {
        return Err(ToolError::Custom("No repository"));
    };

    let Some(author) = parsed
        .get(WORKSPACE)
        .and_then(|v| v.as_table())
        .and_then(|v| v.get(PACKAGE))
        .and_then(|v| v.as_table())
        .and_then(|v| v.get("authors"))
        .and_then(|v| v.as_array())
        .and_then(|a| a.first())
        .and_then(|v| v.as_str())
    else {
        return Err(ToolError::Custom("No authors"));
    };

    let Some(description) = parsed
        .get(PACKAGE)
        .and_then(|v| v.as_table())
        .and_then(|v| v.get("description"))
        .and_then(|v| v.as_str())
    else {
        return Err(ToolError::Custom("No decription"));
    };

    let Some(name) = parsed
        .get(PACKAGE)
        .and_then(|v| v.as_table())
        .and_then(|v| v.get("name"))
        .and_then(|v| v.as_str())
    else {
        return Err(ToolError::Custom("No name"));
    };

    info!("version {:?}", version);

    let mut builder = PackageBuilder::new(
        name,
        version,
        license,
        env::consts::ARCH,
        "A GUI systems manager",
    );

    info!("build rpm");
    builder
        .epoch(0)
        .description(description)
        .url(repository)
        .packager(author)
        .requires(Dependency::any("libgtextutils"))
        .requires(Dependency::greater_eq("gtk4", "4.21.0"))
        .requires(Dependency::greater_eq("libadwaita", "1.8.0"))
        .requires(Dependency::greater_eq("systemd-libs", "257"))
        .requires(Dependency::greater_eq("gtksourceview5", "5.16"));
    // .with_file(
    //     "LICENSE",
    //     FileOptions::new("LICENSE").permissions(0o644).license(),
    // )?
    // .with_dir("target/locale", "/usr/share/locale/", |o| {
    //     o.permissions(0o644)
    // })?;

    add_file(
        &mut builder,
        "target/release/sysd-manager",
        "/usr/bin/",
        0o755,
    )?;
    add_file(
        &mut builder,
        "target/release/sysd-manager-proxy",
        "/usr/bin/",
        0o755,
    )?;
    add_file(
        &mut builder,
        "data/icons/hicolor/scalable/apps/io.github.plrigaux.sysd-manager.svg",
        "/usr/share/icons/hicolor/scalable/apps/",
        0o644,
    )?;
    add_file(
        &mut builder,
        "data/schemas/io.github.plrigaux.sysd-manager.gschema.xml",
        "/usr/share/glib-2.0/schemas/",
        0o644,
    )?;
    add_file(
        &mut builder,
        "target/loc/io.github.plrigaux.sysd-manager.desktop",
        "/usr/share/applications/",
        0o644,
    )?;
    add_file(
        &mut builder,
        "target/loc/io.github.plrigaux.sysd-manager.metainfo.xml",
        "/usr/share/metainfo/",
        0o644,
    )?;
    add_file(
        &mut builder,
        "data/metainfo/io.github.plrigaux.sysd-manager.releases.xml",
        "/usr/share/metainfo/releases/",
        0o644,
    )?;
    add_file(
        &mut builder,
        "target/loc/io.github.plrigaux.SysDManager.policy",
        "/usr/share/polkit-1/actions/",
        0o644,
    )?;
    add_file(
        &mut builder,
        "sysd-manager-proxy/data/50-io.github.plrigaux.SysDManager.rules",
        "/usr/share/polkit-1/rules.d/",
        0o644,
    )?;
    add_file(
        &mut builder,
        "target/generated/sysd-manager-proxy.service",
        "/usr/lib/systemd/system/",
        0o644,
    )?;
    add_file(
        &mut builder,
        "target/generated/io.github.plrigaux.SysDManager.conf",
        "/usr/share/dbus-1/system.d/",
        0o644,
    )?;

    const VALUE: &str = "target/locale";
    let locale = PathBuf::from(VALUE);
    let mut results = Vec::new();
    find_files_recursive(&locale, &mut results)?;

    for mo in results {
        let dest = PathBuf::from("/usr/share/locale");
        let source = mo.to_string_lossy().to_string();

        let stripped = mo
            .strip_prefix(VALUE)
            .map_err(|_e| ToolError::Custom("strip error"))?;
        let dest = dest.join(stripped).to_string_lossy().to_string();

        info!("Try {} {}", source, dest);
        builder.with_file(source, FileOptions::new(dest).permissions(0o644))?;
    }

    let pout = format!("target/sysd-manager_{version}_{}.rpm", env::consts::ARCH);
    info!("Building: {pout}");
    let pkg = builder.build()?;

    pkg.write_file(&pout)?;

    info!("Build: {pout}");
    inspect(pkg)?;
    Ok(())
}

fn find_files_recursive(path: &Path, results: &mut Vec<std::path::PathBuf>) -> std::io::Result<()> {
    if path.is_dir() {
        for entry in std::fs::read_dir(path)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_dir() {
                find_files_recursive(&path, results)?;
            } else {
                results.push(path);
            }
        }
    }
    Ok(())
}

fn add_file(
    builder: &mut PackageBuilder,
    source: &str,
    dest: &str,
    permission: u16,
) -> Result<(), rpm::Error> {
    debug!("source {} dest {}", source, dest);
    let r = if dest.ends_with('/') {
        let file_name = if let Some((_, r)) = source.rsplit_once('/') {
            r
        } else {
            ""
        };

        builder.with_file(
            source,
            FileOptions::new(format!("{dest}{file_name}")).permissions(permission),
        )
    } else {
        builder.with_file(source, FileOptions::new(dest).permissions(permission))
    };

    r.map(|_| ())
}

fn inspect(pkg: Package) -> Result<(), ToolError> {
    let meta = pkg.metadata;
    let nevra = meta.get_nevra()?;
    debug!("Name        : {}", nevra.name());
    debug!("Epoch       : {}", nevra.epoch());
    debug!("Version     : {}", nevra.version());
    debug!("Release     : {}", nevra.release());
    debug!("Architecture: {}", nevra.arch());

    if let Ok(summary) = meta.get_summary() {
        debug!("Summary     : {summary}");
    }
    if let Ok(description) = meta.get_description() {
        debug!("Description : {description}");
    }
    if let Ok(license) = meta.get_license() {
        debug!("License     : {license}");
    }
    if let Ok(url) = meta.get_url() {
        debug!("URL         : {url}");
    }
    if let Ok(vendor) = meta.get_vendor() {
        debug!("Vendor      : {vendor}");
    }
    if let Ok(packager) = meta.get_packager() {
        debug!("Packager    : {packager}");
    }
    if let Ok(build_host) = meta.get_build_host() {
        debug!("Build Host  : {build_host}");
    }
    if let Ok(build_time) = meta.get_build_time() {
        debug!("Build Time  : {build_time}");
    }
    if let Ok(source_rpm) = meta.get_source_rpm() {
        debug!("Source RPM  : {source_rpm}");
    }
    if let Ok(size) = meta.get_installed_size() {
        debug!("Size        : {size}");
    }

    for (label, deps) in [
        ("Provides", meta.get_provides()),
        ("Requires", meta.get_requires()),
        ("Conflicts", meta.get_conflicts()),
        ("Obsoletes", meta.get_obsoletes()),
        ("Recommends", meta.get_recommends()),
        ("Suggests", meta.get_suggests()),
    ] {
        if let Ok(deps) = deps
            && !deps.is_empty()
        {
            debug!("{label}:");
            for dep in &deps {
                debug!("  {dep}");
            }
        }
    }

    if let Ok(entries) = meta.get_file_entries()
        && !entries.is_empty()
    {
        debug!("Files ({}):", entries.len());
        for entry in &entries {
            debug!("  {}", entry.path().display());
        }
    }

    Ok(())
}
