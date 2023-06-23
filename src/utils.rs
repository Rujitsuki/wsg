use std::collections::hash_map::DefaultHasher;
use std::error::Error;
use std::fs;
use std::fs::File;
use std::hash::{Hash, Hasher};
use std::io::Write;
use std::ops::Add;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};
use crate::garbage::{GarbageRecognizer, GarbageRecognizerResult};
use base64::{Engine as _, engine::general_purpose};

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

pub fn cache_garbage_result_vec(from_path: &Path, list: &Vec<GarbageRecognizerResult>) -> Result<PathBuf, Box<dyn Error>> {
    let path_hash = generate_base64_from_path(&from_path);
    let cache_dir_path = std::env::temp_dir().join("wsg/");
    let cache_file_path = cache_dir_path.join(path_hash);

    if !cache_dir_path.exists() {
        fs::create_dir(cache_dir_path)?;
    }

    if cache_file_path.exists() && cache_file_path.is_file() {
        let metadata = cache_file_path.metadata()?;
        let current_time = SystemTime::now();
        let estimated_time = metadata.created()?.add(Duration::from_secs(60 * 5));
        if current_time < estimated_time {
            return Ok(cache_file_path);
        }
    }

    // Generate HashFile
    if cache_file_path.exists() {
        fs::remove_file(&cache_file_path)?;
    }
    let mut file = File::create(&cache_file_path)?;
    let json_string = serde_json::to_string_pretty(&list)?;
    file.write_all(json_string.as_bytes())?;

    Ok(cache_file_path)
}

fn generate_base64_from_path(p: &Path) -> String {
    let bytes = {
        let mut hasher = DefaultHasher::new();
        p.hash(&mut hasher);
        hasher.finish().to_be_bytes()
    };
    general_purpose::STANDARD_NO_PAD.encode(&bytes)
}