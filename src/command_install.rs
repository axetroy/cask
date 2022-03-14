#![deny(warnings)]

use crate::cask;
use crate::extractor;
use crate::formula;
use crate::symlink;
use crate::util;
use crate::util::iso8601;

use std::fs;
use std::fs::File;
use std::io;
use std::io::Write;
use std::time::SystemTime;

use eyre::Report;
use sha2::{Digest, Sha256};

pub async fn install(
    cask: cask::Cask,
    package_name: &str,
    version: Option<&str>,
) -> Result<(), Report> {
    eprintln!("Fetching {} formula...", package_name);

    let package_formula = formula::fetch(&cask, package_name, false)?;

    let download_version = {
        if let Some(v) = version {
            if !package_formula.package.versions.contains(&v.to_string()) {
                Err(eyre::format_err!(
                    "can not found version '{}' of formula",
                    v
                ))
            } else {
                Ok(v.to_owned())
            }
        } else if package_formula.package.versions.is_empty() {
            Err(eyre::format_err!("can not found any version of formula"))
        } else {
            Ok(package_formula.package.versions[0].clone())
        }
    }?;

    // init formula folder
    cask.init_package(&package_formula.package.name)?;

    let package_dir = cask.package_dir(&package_formula.package.name);

    let download_target = package_formula.get_current_download_url(&download_version)?;

    let tar_file_path = cask
        .package_version_dir(&package_formula.package.name)
        .join(format!("{}.{}", &download_version, download_target.ext));

    util::download(&download_target.url, &tar_file_path).await?;

    if let Some(checksum) = download_target.checksum {
        let mut file = File::open(&tar_file_path)?;
        let mut hasher = Sha256::new();
        io::copy(&mut file, &mut hasher)?;
        drop(file);
        let hash = format!("{:x}", hasher.finalize());
        if hash != checksum {
            fs::remove_file(tar_file_path)?;
            return Err(eyre::format_err!(
                "The file SHA256 is '{}' but expect '{}'",
                hash,
                checksum
            ));
        }
    }

    #[cfg(target_family = "unix")]
    let executable_name = package_formula.package.bin.clone();
    #[cfg(target_family = "windows")]
    let executable_name = format!("{}.exe", &package_formula.package.bin);

    let output_file_path =
        extractor::extract(&tar_file_path, &executable_name, &package_dir.join("bin"))?;

    // create symlink to $CASK_ROOT/bin
    {
        let symlink_file = cask.bin_dir().join(&package_formula.package.bin);

        symlink::symlink(&output_file_path, &symlink_file)?;
    }

    // init Cask information in Cask.toml
    {
        let file_path = &package_dir.join("Cask.toml");

        let mut formula_file = File::create(&file_path)?;

        formula_file.write_all(
            format!(
                r#"# The file is generated by Cask. DO NOT MODIFY IT.
[cask]
name = "{}"
created_at = "{}"
version = "{}"
repository = "{}"

"#,
                package_formula.package.name,
                iso8601(&SystemTime::now()),
                download_version,
                formula::get_formula_git_url(package_name)
            )
            .as_str()
            .as_bytes(),
        )?;
        formula_file.write_all(package_formula.get_file_content().as_bytes())?;
    }

    eprintln!(
        "The package '{} {}' has been installed!",
        &package_formula.package.name, download_version
    );

    Ok(())
}
