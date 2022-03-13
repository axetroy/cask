#![deny(warnings)]

use core::result::Result;
use std::fs::{self, File};
use std::io;
use std::path::{Path, PathBuf};

use eyre::Report;
use libflate::gzip::Decoder as GzDecoder;
use tar::Archive;

pub fn extract(
    tar_file_path: &Path,
    extract_file_name: &str,
    dest_dir: &Path,
) -> Result<PathBuf, Report> {
    let tar_file_name = tar_file_path.file_name().unwrap().to_str().unwrap();
    let output_file_path = dest_dir.join(extract_file_name);
    let mut binary_found = false;

    if tar_file_name.ends_with(".tar.gz") {
        let tar_file = File::open(&tar_file_path)?;
        let input = GzDecoder::new(&tar_file)?;
        let mut archive = Archive::new(input);

        archive.set_unpack_xattrs(true);
        archive.set_overwrite(true);
        archive.set_preserve_permissions(true);
        archive.set_preserve_mtime(true);

        let files = archive.entries()?;

        for entry in files {
            let mut file = entry?;

            let file_path = file.path()?;

            if let Some(file_name) = file_path.file_name() {
                if file_name.to_str().unwrap() == extract_file_name {
                    binary_found = true;
                    file.unpack(&output_file_path)?;
                    break;
                }
            }
        }

        if !binary_found {
            Err(eyre::format_err!(
                "can not found binary file '{}' in tar",
                extract_file_name
            ))
        } else {
            Ok(output_file_path)
        }
    } else if tar_file_name.ends_with(".tar") {
        let tar_file = File::open(&tar_file_path)?;
        let mut archive = Archive::new(tar_file);

        archive.set_unpack_xattrs(true);
        archive.set_overwrite(true);
        archive.set_preserve_permissions(true);
        archive.set_preserve_mtime(true);

        let files = archive.entries()?;

        for entry in files {
            let mut file = entry?;

            let file_path = file.path()?;

            if let Some(file_name) = file_path.file_name() {
                if file_name.to_str().unwrap() == extract_file_name {
                    binary_found = true;
                    file.unpack(&output_file_path)?;
                    break;
                }
            }
        }

        if !binary_found {
            Err(eyre::format_err!(
                "can not found binary file '{}' in tar",
                extract_file_name
            ))
        } else {
            Ok(output_file_path)
        }
    } else if tar_file_name.ends_with(".zip") {
        let tar_file = File::open(&tar_file_path)?;
        let mut archive = zip::ZipArchive::new(tar_file).unwrap();

        for i in 0..archive.len() {
            let mut file = archive.by_index(i).unwrap();

            if file.is_dir() {
                continue;
            }

            if file.name() == extract_file_name {
                binary_found = true;
                let mut output_file = fs::File::create(&output_file_path)?;
                io::copy(&mut file, &mut output_file)?;

                // Get and Set permissions
                #[cfg(unix)]
                {
                    use std::os::unix::prelude::PermissionsExt;

                    if let Some(mode) = file.unix_mode() {
                        fs::set_permissions(&output_file_path, fs::Permissions::from_mode(mode))?;
                    };
                };

                break;
            }
        }

        if !binary_found {
            Err(eyre::format_err!(
                "can not found binary file '{}' in tar",
                extract_file_name
            ))
        } else {
            Ok(output_file_path)
        }
    } else {
        Err(eyre::format_err!(
            "Can not extract file from file '{}'",
            tar_file_name
        ))
    }
}

#[cfg(test)]
mod tests {
    use crate::extractor;
    use std::{env, fs};

    #[test]
    fn test_extract_tar_test() {
        let extractor_dir = env::current_dir()
            .unwrap()
            .join("fixtures")
            .join("extractor");

        let tar_file_path = extractor_dir.join("test_1.tar");

        let dest_dir = extractor_dir;

        let extracted_file_path = extractor::extract(&tar_file_path, "test_1", &dest_dir).unwrap();

        let meta = fs::metadata(&extracted_file_path).unwrap();

        assert_eq!(meta.len(), 12);

        let content = fs::read_to_string(&extracted_file_path).unwrap();

        assert_eq!(content, "hello world\n");
    }

    #[test]
    fn test_extract_zip() {
        let extractor_dir = env::current_dir()
            .unwrap()
            .join("fixtures")
            .join("extractor");

        let tar_file_path = extractor_dir.join("test.zip");

        let dest_dir = extractor_dir;

        let extracted_file_path = extractor::extract(&tar_file_path, "test", &dest_dir).unwrap();

        let meta = fs::metadata(&extracted_file_path).unwrap();

        assert_eq!(meta.len(), 12);

        let content = fs::read_to_string(&extracted_file_path).unwrap();

        assert_eq!(content, "hello world\n");
    }

    #[test]
    fn test_extract_tar_if_bin_not_exist() {
        let extractor_dir = env::current_dir()
            .unwrap()
            .join("fixtures")
            .join("extractor");

        let tar_file_path = extractor_dir.join("test.tar");

        let dest_dir = extractor_dir;

        let r = extractor::extract(&tar_file_path, "not_exist", &dest_dir);

        assert!(r.is_err());
    }

    #[test]
    fn test_extract_tar_gz() {
        let extractor_dir = env::current_dir()
            .unwrap()
            .join("fixtures")
            .join("extractor");

        let tar_file_path = extractor_dir.join("test.tar.gz");

        let dest_dir = extractor_dir;

        let extracted_file_path = extractor::extract(&tar_file_path, "test", &dest_dir).unwrap();

        let meta = fs::metadata(&extracted_file_path).unwrap();

        assert_eq!(meta.len(), 12);

        let content = fs::read_to_string(&extracted_file_path).unwrap();

        assert_eq!(content, "hello world\n");
    }

    #[test]
    fn test_extract_tar_gz_with_prune() {
        let extractor_dir = env::current_dir()
            .unwrap()
            .join("fixtures")
            .join("extractor");

        let tar_file_path = extractor_dir.join("prune_darwin_amd64.tar.gz");

        let dest_dir = extractor_dir;

        let extracted_file_path = extractor::extract(&tar_file_path, "prune", &dest_dir).unwrap();

        let meta = fs::metadata(&extracted_file_path).unwrap();

        assert_eq!(meta.len(), 137_656);
    }
}
