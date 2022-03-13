#![deny(warnings)]

use crate::cask;
use crate::extractor;
use crate::formula;
use crate::util;
use crate::util::iso8601;

use std::fs;
use std::fs::File;
use std::io::Write;
use std::time::SystemTime;

use eyre::Report;

pub async fn install(
    cask: cask::Cask,
    package_name: &str,
    version: Option<&str>,
) -> Result<(), Report> {
    let package_formula = formula::fetch(package_name)?;

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
        } else if let Some(v) = &package_formula.package.version {
            if !package_formula.package.versions.contains(v) {
                Err(eyre::format_err!(
                    "can not found version '{}' of formula",
                    v
                ))
            } else {
                Ok(v.clone())
            }
        } else if package_formula.package.versions.is_empty() {
            Err(eyre::format_err!("can not found any version of formula"))
        } else {
            Ok(package_formula.package.versions[0].clone())
        }
    }?;

    // init formula folder
    cask.init_package(package_name)?;

    let package_dir = cask.package_dir(package_name);

    // init Cask information in Cask.toml
    {
        let file_path = &package_dir.join("Cask.toml");

        let mut formula_file = File::create(&file_path)?;

        formula_file.write_all(
            format!(
                r#"# The file is generated by Cask. DO NOT MODIFY IT.
[cask]
package_name = "{}"
created_at = "{}"
version = "{}"

"#,
                package_name,
                iso8601(&SystemTime::now()),
                download_version
            )
            .as_str()
            .as_bytes(),
        )?;
        formula_file.write_all(package_formula.get_file_content().as_bytes())?;
    }

    let url = package_formula.get_current_download_url(&download_version)?;

    let tar_file_path = &cask
        .package_version_dir(package_name)
        .join(format!("{}.tar.gz", &download_version));

    util::download(&url, tar_file_path).await?;

    #[cfg(target_family = "unix")]
    let executable_name = package_formula.package.bin.clone();
    #[cfg(target_family = "windows")]
    let executable_name = format!("{}.exe", &package_formula.package.bin);

    let output_file_path =
        extractor::extract(tar_file_path, &executable_name, &package_dir.join("bin"))?;

    // create symlink to $CASK_ROOT/bin
    {
        let symlink_file = cask.bin_dir().join(executable_name);
        if symlink_file.exists() {
            fs::remove_file(&symlink_file)?;
        }

        #[cfg(target_family = "unix")]
        std::os::unix::fs::symlink(output_file_path, &symlink_file)?;
        #[cfg(target_family = "windows")]
        std::os::windows::fs::symlink_file(output_file_path, &symlink_file)?;
    }

    Ok(())
}
