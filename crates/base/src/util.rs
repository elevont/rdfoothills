// SPDX-FileCopyrightText: 2024 Robin Vobruba <hoijui.quaero@gmail.com>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use std::ffi::OsStr;
// #[cfg(not(feature = "async"))]
// use std::fs;
use std::io;
use std::path::Path as StdPath;
#[cfg(feature = "async")]
use tokio::fs;
#[cfg(feature = "url")]
use {once_cell::sync::Lazy, regex::Regex, url::Url};

#[cfg(feature = "url")]
pub static NON_BASIC_CHARS: Lazy<Regex> = Lazy::new(|| Regex::new(r"[^a-zA-Z0-9]").unwrap());
#[cfg(feature = "url")]
pub static MULTI_UNDERSCORES: Lazy<Regex> = Lazy::new(|| Regex::new(r"__+").unwrap());

#[cfg(feature = "url")]
pub fn url2fname(url: &Url) -> String {
    let url_str = url.as_str();
    let url_cleaned = NON_BASIC_CHARS.replace_all(url_str, "_");
    let url_nameified = MULTI_UNDERSCORES.replace_all(&url_cleaned, "_");
    url_nameified.into()
}

fn handle_create_dir_res<P: AsRef<StdPath> + Send + ?Sized>(
    dir: &P,
    create_dir_res: io::Result<()>,
) {
    create_dir_res
        .map_err(|err| {
            panic!(
                "Failed to create directory `{}`: {err}",
                dir.as_ref().display()
            )
        })
        .unwrap();
}

/// Create a directory if it does not yet exist.
/// There is no error if the directory already exists.
///
/// # Panics
///
/// If the directory cannot be created due to an IO- or permission-error.
pub fn create_dir<P: AsRef<StdPath> + Send>(dir: &P) {
    handle_create_dir_res(dir, create_dir_res(dir.as_ref()));
}

/// Create a directory if it does not yet exist.
/// There is no error if the directory already exists.
///
/// # Panics
///
/// If the directory cannot be created due to an IO- or permission-error.
#[cfg(feature = "async")]
pub async fn create_dir_async<P: AsRef<StdPath> + Send>(dir: P) {
    handle_create_dir_res(dir.as_ref(), create_dir_res_async(dir.as_ref()).await);
}

fn remap_create_dir_res(create_dir_res: io::Result<()>) -> io::Result<()> {
    create_dir_res.or_else(|err| {
        if err.kind() == io::ErrorKind::AlreadyExists {
            Ok(())
        } else {
            Err(err)
        }
    })
}

/// Create a directory if it does not yet exist.
/// There is no error if the directory already exists.
///
/// # Errors
///
/// If the directory cannot be created due to an IO- or permission-error.
pub fn create_dir_res<P: AsRef<StdPath> + Send>(dir: P) -> io::Result<()> {
    remap_create_dir_res(std::fs::create_dir_all(dir))
}

/// Create a directory if it does not yet exist.
/// There is no error if the directory already exists.
///
/// # Errors
///
/// If the directory cannot be created due to an IO- or permission-error.
#[cfg(feature = "async")]
pub async fn create_dir_res_async<P: AsRef<StdPath> + Send>(dir: P) -> io::Result<()> {
    remap_create_dir_res(fs::create_dir_all(dir).await)
}

fn report_err_if_not_a_file(file_path: &StdPath) -> io::Result<bool> {
    Err(io::Error::new(
        io::ErrorKind::Other,
        format!(
            "Should be a file, but is not: '{}' - possible solution: delete it",
            file_path.display()
        ),
    ))
}

/// Checks whether the given path exists and is a file.
///
/// # Errors
///
/// - If the path does not exist.
/// - If the path is not a file.
/// - If there is a permission problem.
/// - If there is an IO error.
pub fn look_for_file(file_path: &StdPath) -> io::Result<bool> {
    let path_exists = StdPath::try_exists(file_path)?;
    if path_exists && !std::fs::metadata(file_path)?.is_file() {
        return report_err_if_not_a_file(file_path);
    }
    Ok(path_exists)
}

/// Checks whether the given path exists and is a file.
///
/// # Errors
///
/// - If the path does not exist.
/// - If the path is not a file.
/// - If there is a permission problem.
/// - If there is an IO error.
#[cfg(feature = "async")]
pub async fn look_for_file_async(file_path: &StdPath) -> io::Result<bool> {
    let path_exists = fs::try_exists(&file_path).await?;
    if path_exists && !fs::metadata(&file_path).await?.is_file() {
        return report_err_if_not_a_file(file_path);
    }
    Ok(path_exists)
}

/// Ensures the provided dir exists.
/// Returns whether it was created.
///
/// # Errors
///
/// - if Checking if the directory exists fails.
/// - if Creating the directory fails.
pub fn ensure_dir_exists(dir_path: &StdPath) -> io::Result<bool> {
    let dir_path_exists = std::path::Path::try_exists(dir_path)?;
    if dir_path_exists {
        if !std::fs::metadata(dir_path)?.is_dir() {
            return Err(io::Error::new(
                // io::ErrorKind::NotADirectory,
                io::ErrorKind::Other,
                format!("Should be an ontology cache directory, but is not a directory: '{}' - possible solution: delete it", dir_path.display())));
        }
    } else {
        std::fs::create_dir_all(dir_path)?;
    }
    Ok(!dir_path_exists)
}

/// Ensures the provided dir exists.
/// Returns whether it was created.
///
/// # Errors
///
/// - if Checking if the directory exists fails.
/// - if Creating the directory fails.
#[cfg(feature = "async")]
pub async fn ensure_dir_exists_async(dir_path: &StdPath) -> io::Result<bool> {
    let dir_path_exists = fs::try_exists(&dir_path).await?;
    if dir_path_exists {
        if !fs::metadata(&dir_path).await?.is_dir() {
            return Err(io::Error::new(
                // io::ErrorKind::NotADirectory,
                io::ErrorKind::Other,
                format!("Should be an ontology cache directory, but is not a directory: '{}' - possible solution: delete it", dir_path.display())));
        }
    } else {
        fs::create_dir_all(&dir_path).await?;
    }
    Ok(!dir_path_exists)
}

pub fn extract_file_ext(file: &StdPath) -> Option<&str> {
    file.extension().and_then(OsStr::to_str)
}
