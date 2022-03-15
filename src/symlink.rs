use eyre::Report;
use std::fs;
use std::fs::File;
use std::io::Write;
use std::path::Path;

pub fn symlink(src: &Path, dest: &Path) -> Result<(), Report> {
    if cfg!(unix) {
        if dest.exists() {
            fs::remove_file(&dest)?;
        }
        #[cfg(unix)]
        std::os::unix::fs::symlink(&src, &dest)?;
    } else {
        // instead of create a symlink in windows
        // we should generate a bat/shell file like this
        let filename = (*dest)
            .to_path_buf()
            .file_name()
            .unwrap()
            .to_str()
            .unwrap()
            .to_owned();

        // generate a bat
        {
            let bat_file_name = filename.clone() + ".bat";

            let bat_file_path = dest.parent().unwrap().join(bat_file_name);

            let mut bat_file = File::create(bat_file_path)?;

            let bat_script = include_str!("./script/exe.bat")
                .replace("{filepath}", src.as_os_str().to_str().unwrap());

            bat_file.write_all(bat_script.as_str().as_bytes())?;
        }

        // generate a shell
        {
            let shell_file_name = &filename;

            let shell_file_path = dest.parent().unwrap().join(shell_file_name);

            let mut shell_file = File::create(shell_file_path)?;

            let bat_script = include_str!("./script/exe.sh")
                .replace("{filepath}", src.as_os_str().to_str().unwrap());

            shell_file.write_all(bat_script.as_str().as_bytes())?;
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use crate::symlink;
    use std::env;

    #[test]
    fn test_symlink() {
        let cwd = env::current_dir().unwrap();

        let src = cwd
            .join("fixtures")
            .join("symlink")
            .join("src")
            .join("test");

        let dest = cwd
            .join("fixtures")
            .join("symlink")
            .join("dest")
            .join("test");

        symlink::symlink(&src, &dest).unwrap();

        #[cfg(unix)]
        assert!(dest.is_symlink());
        #[cfg(windows)]
        {
            assert!(!dest.is_symlink());
            assert!(dest.is_file());

            let bat = dest.parent().unwrap().join("test.bat");

            assert!(!bat.is_symlink());
            assert!(bat.is_file());
        }
    }
}
