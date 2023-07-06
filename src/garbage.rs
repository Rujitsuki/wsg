use crate::error::GarbageError;
use crate::utils::{dir_size, read_garbage_result_vec_cache};
use crate::AppState;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::fmt::{Display, Formatter};
use std::num::ParseIntError;
use std::path::{Path, PathBuf};
use std::str::FromStr;
use std::{fmt, fs, io};
use walkdir::WalkDir;

#[derive(Eq, PartialEq, Hash, Debug)]
pub struct GarbageRecognizer {
    pub name: String,
    pub recognize: Vec<FileType>,
    pub delete: Vec<FileType>,
}

impl GarbageRecognizer {
    pub fn new<S: Into<String>>(
        name: S,
        recognize: Option<Vec<FileType>>,
        delete: Option<Vec<FileType>>,
    ) -> Self {
        Self {
            name: name.into(),
            recognize: recognize.unwrap_or_default(),
            delete: delete.unwrap_or_default(),
        }
    }
}

#[derive(Eq, PartialEq, Hash, Debug)]
pub enum FileType {
    File(String),
    Directory(String),
}

#[derive(Serialize, Deserialize, Debug)]
pub struct GarbageRecognizerResult {
    pub index: GarbageIndex,
    pub recognizer_name: String,
    pub directory: PathBuf,
    pub size: u64,
    pub deletable: Vec<PathBuf>,
}

#[derive(Serialize, Deserialize, PartialEq, Clone, Debug)]
pub enum GarbageIndex {
    Id(u32),
    All,
}

impl FromStr for GarbageIndex {
    type Err = ParseIntError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let ident = match s {
            "all" => GarbageIndex::All,
            _ => GarbageIndex::Id(u32::from_str(s)?),
        };

        Ok(ident)
    }
}

impl Display for GarbageIndex {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            GarbageIndex::Id(id) => write!(f, "{}", id),
            GarbageIndex::All => write!(f, "all"),
        }
    }
}

pub fn find_garbage_in_directory(
    path: &Path,
    state: &AppState,
) -> Result<Vec<GarbageRecognizerResult>, GarbageError> {
    let mut ignored_subdirectories = HashSet::<PathBuf>::new();
    let mut results = Vec::<GarbageRecognizerResult>::new();
    let mut ident_counter = 0;

    for entry in WalkDir::new(path).follow_links(false) {
        let entry = entry?;
        let metadata = entry.metadata()?;
        let entry_path = entry.path();

        if metadata.is_file() {
            continue;
        }

        if ignored_subdirectories
            .iter()
            .any(|ignored_subdirectory| entry_path.starts_with(ignored_subdirectory))
        {
            continue;
        }

        for recognizer in state.garbage_recognizer.iter() {
            let mut deletable_files = Vec::new();
            let mut directory_size = 0;

            let contains_recognitions = recognizer.recognize.iter().any(|recognition| {
                let file_type_path = match recognition {
                    FileType::File(value) => value,
                    FileType::Directory(value) => value,
                };
                let file_path = entry_path.join(file_type_path);
                file_path.exists()
            });

            let contains_deletable_content = recognizer.delete.iter().any(|recognition| {
                let file_type_path = match recognition {
                    FileType::File(value) => value,
                    FileType::Directory(value) => value,
                };
                let deletable_content_path = entry_path.join(file_type_path);
                if deletable_content_path.exists() {
                    directory_size = dir_size(&deletable_content_path).unwrap_or_default();
                    ignored_subdirectories.insert(deletable_content_path.clone());
                    deletable_files.push(deletable_content_path.clone());
                    true
                } else {
                    false
                }
            });

            if contains_recognitions && contains_deletable_content {
                let garbage_result = GarbageRecognizerResult {
                    index: GarbageIndex::Id(ident_counter),
                    recognizer_name: recognizer.name.clone(),
                    directory: entry_path.to_path_buf(),
                    size: directory_size,
                    deletable: deletable_files,
                };
                results.push(garbage_result);
                ident_counter += 1;
            }
        }
    }

    Ok(results)
}

pub fn compute_deletable_size_from_garbage_results(results: &Vec<GarbageRecognizerResult>) -> u64 {
    results.iter().map(|entry| &entry.size).sum()
}

pub fn clean_garbage_in_directory(
    path: &Path,
) -> Result<Vec<DeleteOperationSelection>, GarbageError> {
    let result_list: Vec<GarbageRecognizerResult> = read_garbage_result_vec_cache(path, None)?;
    clean_garbage_from_vec(result_list)
}

pub fn clean_garbage_from_vec(
    garbage: Vec<GarbageRecognizerResult>,
) -> Result<Vec<DeleteOperationSelection>, GarbageError> {
    let result: Vec<DeleteOperationSelection> = garbage
        .iter()
        .map(|result| delete_deletable_from_garbage_recognizer_result(&result))
        .collect();

    Ok(result)
}

fn delete_deletable_from_garbage_recognizer_result(
    result: &GarbageRecognizerResult,
) -> DeleteOperationSelection {
    let results: Vec<DeleteOperationResult> = result
        .deletable
        .iter()
        .map(|path| match path.metadata() {
            Ok(metadata) => {
                if metadata.is_dir() {
                    delete_dir(path)
                } else if metadata.is_dir() {
                    delete_file(path)
                } else {
                    DeleteOperationResult::failure(path.to_path_buf(), None)
                }
            }
            Err(e) => {
                DeleteOperationResult::failure(path.to_path_buf(), Some(e.to_string()))
            }
        })
        .collect();

    DeleteOperationSelection::new(result.recognizer_name.to_string(), results)
}

fn delete_dir(path: &Path) -> DeleteOperationResult {
    result_of_deletion(path, fs::remove_dir_all(path))
}

fn delete_file(path: &Path) -> DeleteOperationResult {
    result_of_deletion(path, fs::remove_file(path))
}

fn result_of_deletion(path: &Path, result: io::Result<()>) -> DeleteOperationResult {
    match result {
        Ok(_) => DeleteOperationResult::success(path.to_path_buf()),
        Err(e) => DeleteOperationResult::failure(path.to_path_buf(), Some(e.to_string())),
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct DeleteOperationSelection {
    name: String,
    result: Vec<DeleteOperationResult>,
}

impl DeleteOperationSelection {
    pub fn new<S: Into<String>>(name: S, result: Vec<DeleteOperationResult>) -> Self {
        Self {
            name: name.into(),
            result,
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct DeleteOperationResult {
    path: PathBuf,
    success: bool,
    error_message: Option<String>,
}

impl DeleteOperationResult {
    pub fn success(path: PathBuf) -> Self {
        Self {
            path,
            success: true,
            error_message: None,
        }
    }

    pub fn failure(path: PathBuf, error_message: Option<String>) -> Self {
        Self {
            path,
            success: false,
            error_message,
        }
    }
}

pub fn filter_garbage_from_ids(
    garbage: Vec<GarbageRecognizerResult>,
    ids: &Vec<GarbageIndex>,
) -> Vec<GarbageRecognizerResult> {
    if ids.contains(&GarbageIndex::All) {
        return garbage;
    }

    garbage
        .into_iter()
        .filter(|result| ids.contains(&result.index))
        .collect()
}
