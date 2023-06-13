use std::collections::HashSet;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;
use crate::AppState;
use crate::error::GarbageError;
use crate::utils::dir_size;

#[derive(Eq, PartialEq, Hash, Debug)]
pub struct GarbageRecognizer {
    pub ident: String,
    pub recognize: Vec<FileType>,
    pub delete: Vec<FileType>,
}

impl GarbageRecognizer {
    pub fn new<S: Into<String>>(ident: S, recognize: Option<Vec<FileType>>, delete: Option<Vec<FileType>>) -> Self {
        Self {
            ident: ident.into(),
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

#[derive(Debug)]
pub struct GarbageRecognizerResult {
    pub ident: String,
    pub directory: PathBuf,
    pub size: u64,
    pub deletable: Vec<PathBuf>,
}

pub fn find_garbage_in_directory(path: &Path, state: &AppState) -> Result<Vec<GarbageRecognizerResult>, GarbageError> {
    let mut ignored_subdirectories = HashSet::<PathBuf>::new();
    let mut results = Vec::<GarbageRecognizerResult>::new();

    for entry in WalkDir::new(path).follow_links(false) {
        let entry = entry?;
        let metadata = entry.metadata()?;
        let entry_path = entry.path();

        if metadata.is_file() {
            continue;
        }

        if ignored_subdirectories.iter().any(|ignored_subdirectory| entry_path.starts_with(ignored_subdirectory)) {
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
                    ident: recognizer.ident.clone(),
                    directory: entry_path.to_path_buf(),
                    size: directory_size,
                    deletable: deletable_files,
                };
                results.push(garbage_result);
            }
        }
    }

    Ok(results)
}

