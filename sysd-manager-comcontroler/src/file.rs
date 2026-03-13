use crate::errors::SystemdErrors;
use base::file::{
    create_drop_in_path_file, flatpak_host_file_path, write_on_disk, write_with_priviledge,
};
use std::path::PathBuf;
use tracing::info;

pub(crate) async fn create_drop_in(
    runtime: bool,
    user_session: bool,
    unit_name: &str,
    file_name: &str,
    content: &str,
) -> Result<(), SystemdErrors> {
    let file_path = create_drop_in_path_file(unit_name, runtime, user_session, file_name)?;

    #[cfg(not(any(feature = "flatpak", feature = "appimage")))]
    create_drop_in_io(&file_path, content).await?;

    #[cfg(any(feature = "flatpak", feature = "appimage"))]
    no_proxy::create_drop_in_script(&file_path, content, user_session).await?;

    Ok(())
}

pub async fn save_text_to_file(
    file_path: &str,
    content: &str,
    user_session: bool,
) -> Result<u64, SystemdErrors> {
    let host_file_path = if user_session {
        flatpak_host_file_path(file_path)
    } else {
        PathBuf::from(file_path)
    };

    info!(
        "Try to save content on File: {} with priviledge {}",
        host_file_path.display(),
        !user_session
    );

    Ok(if user_session {
        write_on_disk(&host_file_path, false, content).await?
    //TODO ask if want to force with  priviledge
    } else {
        write_with_priviledge(&host_file_path, content).await?
    })
}

#[cfg(any(feature = "flatpak", feature = "appimage"))]
mod no_proxy {

    use base::{args, file::execute_command};

    use crate::errors::SystemdErrors;
    use std::ffi::OsStr;
    use std::fmt::Write;

    pub(crate) async fn create_drop_in_script(
        file_path: &str,
        content: &str,
        user_session: bool,
    ) -> Result<u64, SystemdErrors> {
        //let file_path = flatpak_host_file_path(file_path);

        //let file_path_str = file_path.to_string_lossy();

        let dir_name = std::path::Path::new(file_path)
            .parent()
            .ok_or(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!("Parent dir of file {:?} is invalid", file_path),
            ))?;

        let mut script = String::new();

        writeln!(script, "echo Start script")?;
        writeln!(script, "echo Create drop-in at {}", file_path)?;
        writeln!(script, "mkdir -vp {}", dir_name.to_string_lossy())?;
        writeln!(script, "cat > {} <<- EOM", file_path)?;
        writeln!(script, "{}", content)?;
        writeln!(script, "EOM")?;
        writeln!(script, "echo End Script")?;

        let r = if user_session {
            script_as_user(&script).await
        } else {
            script_with_priviledge(&script).await
        };
        r.map(|_| content.len() as u64)
    }

    async fn script_as_user(script: &str) -> Result<(), SystemdErrors> {
        let prog_n_args = args!["sh"];
        execute_command(Some(script.as_bytes()), &prog_n_args).await?;
        Ok(())
    }

    async fn script_with_priviledge(script: &str) -> Result<(), SystemdErrors> {
        let prog_n_args = args!["pkexec", "sh"];
        execute_command(Some(script.as_bytes()), &prog_n_args).await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::process::Stdio;
    use std::{fs, path::PathBuf};
    use test_base::init_logs;
    use tokio::io::AsyncBufReadExt;
    use tokio::io::AsyncWriteExt;
    use tokio::io::BufReader;

    #[cfg(feature = "flatpak")]
    use std::fmt::Write;

    use crate::{errors::SystemdErrors, file::write_with_priviledge};

    #[ignore = "writes file with priviledge"]
    #[tokio::test]
    async fn test_write_with_prvi() -> Result<(), SystemdErrors> {
        init_logs();

        let p = PathBuf::from(".").canonicalize()?.join("test.txt");

        let r = write_with_priviledge(&p, "Some text for a test 2").await?;

        info!("Bytes written: {}", r);

        Ok(())
    }

    #[test]
    fn test_canonicalize() {
        let srcdir = PathBuf::from("./src");
        println!("{:?}", fs::canonicalize(&srcdir));

        let solardir = PathBuf::from(".");
        println!("{:?}", fs::canonicalize(&solardir));
    }

    #[tokio::test]
    #[cfg(feature = "flatpak")]
    #[ignore = "writes file with priviledge"]
    async fn test_script() -> Result<(), SystemdErrors> {
        init_logs();

        let mut s = String::new();

        let path = PathBuf::from(".");
        let mut dir_name = fs::canonicalize(&path)?;
        dir_name.push("asdf.d");

        let file_name = "test_out.txt";
        let file_name = dir_name.join(file_name);
        let file_name = file_name.to_string_lossy();

        println!("{}", file_name);

        let content =
            "thet is a test \n abigeg test\n ffffffffffffffffffffffffff\n aaaaaaaaaaaaaaa";

        writeln!(s, "echo TEST file {}", file_name)?;
        writeln!(s, "mkdir -vp {}", dir_name.to_string_lossy())?;
        writeln!(s, "cat > {} <<- EOM", file_name)?;
        writeln!(s, "{}", content)?;
        writeln!(s, "EOM")?;

        script_with_priviledge(&s).await?;

        Ok(())
    }

    #[tokio::test]
    #[cfg(feature = "flatpak")]
    async fn test_create_drop_in_script() -> Result<(), SystemdErrors> {
        init_logs();

        let path = PathBuf::from(".");
        let mut dir_name = fs::canonicalize(&path)?;
        dir_name.push("test_dir.d");

        let file_name = "test_out.txt";
        let file_name = dir_name.join(file_name);
        let file_name = file_name.to_string_lossy();
        info!("{}", file_name);

        let content =
            "thet is a test \n abigeg test\n ffffffffffffffffffffffffff\n aaaaaaaaaaaaaaa";

        create_drop_in_script(&file_name, content, false).await?;

        Ok(())
    }

    #[tokio::test]
    #[cfg(feature = "flatpak")]
    #[ignore = "writes file with priviledge"]
    async fn test_create_drop_in_script_user() -> Result<(), SystemdErrors> {
        init_logs();

        let path = PathBuf::from(".");
        let mut dir_name = fs::canonicalize(&path)?;
        dir_name.push("test_dir_user.d");

        let file_name = "test_out_user.txt";
        let file_name = dir_name.join(file_name);
        let file_name = file_name.to_string_lossy();
        info!("{}", file_name);

        let content =
            "thet is a test \n abigeg test\n ffffffffffffffffffffffffff\n aaaaaaaaaaaaaaa";

        create_drop_in_script(&file_name, content, true).await?;

        Ok(())
    }

    #[ignore = "writes file with priviledge"]
    #[tokio::test]
    async fn test_sort() -> Result<(), Box<dyn std::error::Error>> {
        init_logs();
        let mut cmd = tokio::process::Command::new("sh");

        // Specifying that we want pipe both the output and the input.
        // Similarly to capturing the output, by configuring the pipe
        // to stdin it can now be used as an asynchronous writer.
        cmd.stdout(Stdio::piped());
        cmd.stdin(Stdio::piped());

        let mut child = cmd.spawn().expect("failed to spawn command");

        let stdout = child
            .stdout
            .take()
            .expect("child did not have a handle to stdout");

        let mut stdin = child
            .stdin
            .take()
            .expect("child did not have a handle to stdin");

        let mut reader = BufReader::new(stdout).lines();
        stdin
            .write_all("echo test test".as_bytes())
            .await
            .expect("could not write to stdin");

        // We drop the handle here which signals EOF to the child process.
        // This tells the child process that it there is no more data on the pipe.
        drop(stdin);

        let op = child.wait().await?;

        println!("ExitStatus: {}", op);

        while let Some(line) = reader.next_line().await? {
            info!("Line: {}", line);
        }

        Ok(())
    }
}
