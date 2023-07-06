use crate::error::GarbageError;
use crate::garbage::GarbageRecognizerResult;
use base64::{engine::general_purpose, Engine as _};
use std::collections::hash_map::DefaultHasher;
use std::fs;
use std::fs::File;
use std::hash::{Hash, Hasher};
use std::io::{Read, Write};
use std::ops::Add;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};
use walkdir::WalkDir;

pub fn dir_size(path: impl Into<PathBuf>) -> std::io::Result<u64> {
    let mut dir: fs::ReadDir = fs::read_dir(path.into())?;
    dir.try_fold(0, |acc, file| {
        let file = file?;
        let size = match file.metadata()? {
            data if data.is_dir() => dir_size(file.path())?,
            data => data.len(),
        };
        Ok(acc + size)
    })
}

pub fn format_bytes(bytes: u64) -> String {
    let units = ["B", "kB", "MB", "GB", "TB", "PB", "EB", "ZB", "YB"];
    let mut value = bytes as f64;
    let mut unit_index = 0;

    while value >= 1000.0 && unit_index < units.len() - 1 {
        value /= 1000.0;
        unit_index += 1;
    }

    format!("{:.2} {}", value, units[unit_index])
}

pub fn write_garbage_result_vec_cache(
    from_path: &Path,
    result_list: &Vec<GarbageRecognizerResult>,
    cache_durability: Option<Duration>,
) -> Result<PathBuf, GarbageError> {
    let path_hash = generate_base64_from_path(from_path);
    let cache_dir_path = std::env::temp_dir().join("wsg/");
    let cache_file_path = cache_dir_path.join(path_hash);

    if !cache_dir_path.exists() {
        fs::create_dir(cache_dir_path)?;
    }

    if cache_file_path.exists() && cache_file_path.is_file() {
        let estimated_time = cache_file_path
            .metadata()?
            .modified()?
            .add(cache_durability.unwrap_or(Duration::from_secs(60 * 5)));

        if is_cache_durable(estimated_time) {
            return Ok(cache_file_path);
        }
    }

    let mut file = File::create(&cache_file_path)?;
    let json_string = serde_json::to_string_pretty(result_list)?;
    file.write_all(json_string.as_bytes())?;

    Ok(cache_file_path)
}

pub fn read_garbage_result_vec_cache(
    from_path: &Path,
    cache_durability: Option<Duration>,
) -> Result<Vec<GarbageRecognizerResult>, GarbageError> {
    let path_hash = generate_base64_from_path(from_path);
    let cache_dir_path = std::env::temp_dir().join("wsg/");
    let cache_file_path = cache_dir_path.join(path_hash);

    let mut file = File::open(&cache_file_path)?;
    let estimated_time = file
        .metadata()?
        .modified()?
        .add(cache_durability.unwrap_or(Duration::from_secs(60 * 5)));

    if !is_cache_durable(estimated_time) {
        return Err(GarbageError::InvalidCache);
    }

    let mut json_string = String::new();
    file.read_to_string(&mut json_string)?;

    let result_list: Vec<GarbageRecognizerResult> = serde_json::from_str(&json_string)?;
    Ok(result_list)
}

fn is_cache_durable(estimated_time: SystemTime) -> bool {
    SystemTime::now() < estimated_time
}

pub fn delete_garbage_result_vec_cache(from_path: &Path) -> Result<(), GarbageError> {
    let path_hash = generate_base64_from_path(from_path);
    let cache_dir_path = std::env::temp_dir().join("wsg/");
    let cache_file_path = cache_dir_path.join(path_hash);

    if !cache_file_path.exists() || !cache_file_path.is_file() {
        let error = std::io::Error::from(std::io::ErrorKind::NotFound);
        return Err(GarbageError::IOError(error));
    }

    fs::remove_file(cache_file_path)?;
    Ok(())
}

pub fn delete_all_cache_files() -> Result<(), GarbageError> {
    let cache_dir_path = std::env::temp_dir().join("wsg/");
    for entry in WalkDir::new(cache_dir_path)
        .follow_links(false)
        .max_depth(1)
    {
        let entry = entry?;
        let metadata = entry.metadata()?;
        if metadata.is_file() {
            fs::remove_file(entry.path())?;
        }
    }

    Ok(())
}

fn generate_base64_from_path(p: &Path) -> String {
    let bytes = {
        let mut hasher = DefaultHasher::new();
        p.hash(&mut hasher);
        hasher.finish().to_be_bytes()
    };
    general_purpose::STANDARD_NO_PAD.encode(&bytes)
}
