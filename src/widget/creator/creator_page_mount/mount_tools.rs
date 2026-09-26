use base::file::commander;
use regex::Regex;
use std::{
    collections::BTreeSet,
    fmt::Write,
    path::{Path, PathBuf},
};
use systemd::errors::SystemdErrors;
use tokio::{
    fs::{self, File},
    io::{AsyncBufReadExt, BufReader},
};
use tracing::{debug, error, info, warn};

pub async fn fetch_filesystem_names() -> Result<BTreeSet<String>, SystemdErrors> {
    let mut file_systems_names = BTreeSet::new();
    fetch_kernel_filesystem(&mut file_systems_names).await?;
    if let Err(err) = fetch_module_filesystem(&mut file_systems_names).await {
        warn!("fetch_module_filesystem {:?}", err);
    };

    Ok(file_systems_names)
}

async fn fetch_kernel_filesystem(
    file_systems_names: &mut BTreeSet<String>,
) -> Result<(), SystemdErrors> {
    let file_path = "/proc/filesystems";

    if !PathBuf::from(file_path).exists() {
        error!("{} don't exist!", file_path);
    }

    let file = File::open(file_path).await?;
    let reader = BufReader::new(file);

    let mut lines = reader.lines(); // Iterates over lines efficiently without loading the whole file into RAM

    let re = Regex::new(r"(\w*)\t(\w*)").unwrap();

    while let Some(line) = lines.next_line().await? {
        if let Some(cap) = re.captures(&line) {
            // debug!("cap {} fs {}", &cap[1], &cap[2]);
            file_systems_names.insert(cap[2].to_owned());
        } else {
            warn!("Not capture")
        };
        // println!("{}", line);
    }

    Ok(())
}

async fn fetch_module_filesystem(
    file_systems_names: &mut BTreeSet<String>,
) -> Result<(), SystemdErrors> {
    let mut c = commander(["uname", "-r"], None);
    let output = c.output().await.expect("Failed to execute command");

    let kernel_release = String::from_utf8_lossy(&output.stdout);
    let kernel_release = kernel_release.trim();

    let dir_path = format!("/lib/modules/{}/kernel/fs", kernel_release);

    info!("Module dir_path {dir_path}");

    let mut rd = fs::read_dir(dir_path).await?;

    while let Some(entry) = rd.next_entry().await? {
        let s = entry.file_name();
        let name = s.to_string_lossy().into_owned();
        // debug!("s {s}");
        file_systems_names.insert(name);
    }

    Ok(())
}

pub async fn fetch_resources_to_mount() -> Result<BTreeSet<String>, SystemdErrors> {
    let mut mount_points = BTreeSet::new();
    let dir_path = PathBuf::from("/dev/disk");

    fetch_mount_point_sub(&dir_path, &mut mount_points, "by-uuid", "UUID").await?;
    fetch_mount_point_sub(&dir_path, &mut mount_points, "by-label", "LABEL").await?;

    Ok(mount_points)
}

async fn fetch_mount_point_sub(
    dir_path: &Path,
    mount_points: &mut BTreeSet<String>,
    dir_name: &str,
    short_label: &str,
) -> Result<(), SystemdErrors> {
    let path = dir_path.join(dir_name);
    let mut read_dir = fs::read_dir(&path).await?;
    while let Some(entry) = read_dir.next_entry().await? {
        let file_name = entry.file_name();
        let name = file_name.to_string_lossy().into_owned();

        let long_path = path.join(&name);
        mount_points.insert(long_path.to_string_lossy().to_string());

        let short = format!("{}={}", short_label, name);
        mount_points.insert(short);

        if let Err(err) = fetch_symlink(&path, &long_path, mount_points).await {
            error!("fetch symlink error {} {}", long_path.display(), err)
        };
    }
    Ok(())
}

async fn fetch_symlink(
    path: &Path,
    long_path: &Path,
    mount_points: &mut BTreeSet<String>,
) -> Result<(), SystemdErrors> {
    if long_path.is_symlink() {
        let target = fs::read_link(&long_path).await?;
        debug!("symlink {}", target.display());
        let target = if target.is_relative() {
            let join = path.join(target);
            join.canonicalize()?
        } else {
            target
        };
        debug!("abs {}", target.display());
        mount_points.insert(target.to_string_lossy().to_string());
    };
    Ok(())
}

/// same as https://www.freedesktop.org/software/systemd/man/latest/systemd-escape.html
/// The escaping algorithm operates as follows: given a string, any "/" character is replaced by "-", and all other
/// characters which are not ASCII alphanumerics, ":", "_" or "." are replaced by C-style "\x2d" escapes. In addition,
/// "." is replaced with such a C-style escape when it would appear as the first character in the escaped string.
///
/// When the input qualifies as absolute file system path, this algorithm is extended slightly: the path to the root
/// directory "/" is encoded as single dash "-". In addition, any leading, trailing or duplicate "/" characters are
/// removed from the string before transformation. Example: /foo//bar/baz/ becomes "foo-bar-baz".
///
/// This escaping is fully reversible, as long as it is known whether the escaped string was a path (the unescaping
/// results are different for paths and non-path strings). The systemd-escape(1) command may be used to apply and
/// reverse escaping on arbitrary strings. Use systemd-escape --path to escape path strings, and systemd-escape
/// without --path otherwise.
pub fn escape_path(path: &str) -> String {
    let mut out = String::with_capacity(path.len() + 30);
    //validate proper path

    let mut it = path.chars().peekable();

    while let Some(ch) = it.next() {
        if ch == '/' {
            match it.peek() {
                Some(next_c) if *next_c == '/' => continue,
                Some(_) if !out.is_empty() => out.push('-'),
                None if out.is_empty() => out.push('-'),
                _ => {}
            };
        } else if ch.is_ascii_alphanumeric() || ch == '_' {
            out.push(ch);
        } else {
            let mut buffer = [0u8; 4];
            ch.encode_utf8(&mut buffer);

            for b in buffer.iter().take_while(|b| **b != 0) {
                let _ = write!(out, "\\x{:02x}", b);
            }
        }
    }
    // out.push_str(MOUNT_SUFFIX);
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;
    use systemd::errors::SystemdErrors;
    use tracing::info;

    #[test]
    fn test_escape_path() {
        assert_eq!(
            &escape_path("/Hallöchen, Meister"),
            r"Hall\xc3\xb6chen\x2c\x20Meister"
        );
        assert_eq!(&escape_path("/tmp//waldi/foobar/"), r"tmp-waldi-foobar");
        assert_eq!(&escape_path("/tmp//waldi/////foobar"), r"tmp-waldi-foobar");
        assert_eq!(&escape_path("/mnt/pizza"), r"mnt-pizza");
        assert_eq!(&escape_path("/"), r"-");
        assert_eq!(&escape_path("////test"), "test");
        assert_eq!(&escape_path("/mnt/old_fedo"), "mnt-old_fedo");
    }

    #[tokio::test]
    async fn test_kernel_filesystem() -> Result<(), SystemdErrors> {
        test_base::init_logs();

        let mut file_systems_names = BTreeSet::new();
        fetch_kernel_filesystem(&mut file_systems_names).await?;

        info!("{:?}", file_systems_names);
        Ok(())
    }

    #[test]
    fn test_is_absolute() {
        test_base::init_logs();

        is_absolute("/mnt", true);
        is_absolute("/mnt/../no", true);
        is_absolute("mnt/no", false);
        is_absolute("mnt/../no", false);
    }

    fn is_absolute(path_str: &str, test: bool) {
        let path = PathBuf::from(path_str);
        let abs = path.is_absolute();
        assert_eq!(test, abs, "path: {}", path_str);
    }

    #[tokio::test]
    async fn test_fetch_module_filesystem() -> Result<(), SystemdErrors> {
        test_base::init_logs();

        let mut file_systems_names = BTreeSet::new();
        fetch_module_filesystem(&mut file_systems_names).await?;
        info!("{:?}", file_systems_names);

        Ok(())
    }

    #[tokio::test]
    async fn test_fetch_mount_points() -> Result<(), SystemdErrors> {
        test_base::init_logs();

        let mount_points = fetch_resources_to_mount().await?;
        info!("{:#?}", mount_points);

        Ok(())
    }
}
