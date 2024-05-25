// SPDX-FileCopyrightText: 2024 Robin Vobruba <hoijui.quaero@gmail.com>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use std::ffi::OsStr;
use std::io;
use std::path::Path as StdPath;
use tokio::fs;

pub async fn create_dir<P: AsRef<StdPath> + Send>(dir: P) {
    create_dir_res(dir.as_ref())
        .await
        .map_err(|err| {
            panic!(
                "Failed to create directory `{}`: {err}",
                dir.as_ref().display()
            )
        })
        .unwrap();
}

pub async fn create_dir_res<P: AsRef<StdPath> + Send>(dir: P) -> io::Result<()> {
    fs::create_dir_all(dir).await.or_else(|err| {
        if err.kind() == io::ErrorKind::AlreadyExists {
            Ok(())
        } else {
            Err(err)
        }
    })
}

pub async fn look_for_file(file_path: &StdPath) -> io::Result<bool> {
    let path_exists = tokio::fs::try_exists(&file_path).await?;
    if path_exists
        && !tokio::fs::metadata(&file_path)
            .await?
            // .map_err(|err| format!("Failed to check if directory path '{}' is a directory - '{err}'", dir_path.display()))
            .is_file()
    {
        return Err(io::Error::new(
            io::ErrorKind::Other,
            format!(
                "Should be an file, but is not: '{}' - possible solution: delete it",
                file_path.display()
            ),
        ));
    }
    Ok(path_exists)
}

/// Ensures the provided dir exists.
/// Returns whether it was created.
pub async fn ensure_dir_exists(dir_path: &StdPath) -> io::Result<bool> {
    let dir_path_exists = tokio::fs::try_exists(&dir_path)
        .await
        // .map_err(|err| format!("Failed to check if directory path '{}' exists - '{err}'", dir_path.display()))
        ?;
    if dir_path_exists {
        if !tokio::fs::metadata(&dir_path)
            .await?
            // .map_err(|err| format!("Failed to check if directory path '{}' is a directory - '{err}'", dir_path.display()))
            .is_dir()
        {
            return Err(io::Error::new(
                // io::ErrorKind::NotADirectory,
                io::ErrorKind::Other,
                format!("Should be an ontology cache directory, but is not a directory: '{}' - possible solution: delete it", dir_path.display())));
        }
    } else {
        tokio::fs::create_dir_all(&dir_path)
            .await
            // .map_err(|err| format!("Failed to create directory '{}' is a directory - '{err}'", dir_path.display()))
            ?;
    }
    Ok(!dir_path_exists)
}

pub fn extract_file_ext(file: &StdPath) -> Option<&str> {
    file.extension().and_then(OsStr::to_str)
}
