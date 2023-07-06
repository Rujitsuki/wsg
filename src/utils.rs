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

#[cfg(test)]
mod tests {
    use crate::garbage::{GarbageIndex, GarbageRecognizerResult};
    use crate::utils::{
        delete_garbage_result_vec_cache, dir_size, format_bytes, generate_base64_from_path,
        is_cache_durable, read_garbage_result_vec_cache, write_garbage_result_vec_cache,
    };
    use std::env::temp_dir;
    use std::fs;
    use std::fs::File;
    use std::io::Write;
    use std::ops::{Add, Sub};
    use std::path::{Path, PathBuf};
    use std::time::{Duration, SystemTime};

    #[test]
    fn test_dir_size() {
        let temp_dir = temp_dir().join("wsg_dev");
        fs::create_dir_all(&temp_dir).expect("Failed to create temporary wsg_dev directory");

        let file01 = &temp_dir.join("01");
        let file02 = &temp_dir.join("02");

        File::create(file01)
            .expect("Failed to create test file")
            .write_all(vec![0; 1_000_000].as_slice())
            .expect("Can't write test bytes to file");
        File::create(file02)
            .expect("Failed to create test file")
            .write_all(vec![0; 1_500_000].as_slice())
            .expect("Can't write test bytes to file");

        let result = dir_size(&temp_dir);
        assert!(result.is_ok());

        let result = result.unwrap();
        assert_eq!(result, 2_500_000);

        fs::remove_dir_all(&temp_dir).expect("Can't delete wsg_dev directory");
    }

    #[test]
    fn test_format_bytes() {
        let test_cases = [
            (0, "0.00 B"),
            (100, "100.00 B"),
            (1000, "1.00 kB"),
            (1000_00, "100.00 kB"),
            (1000_000, "1.00 MB"),
            (1000_000_00, "100.00 MB"),
            (1000_000_000, "1.00 GB"),
            (1000_000_000_00, "100.00 GB"),
            (1000_000_000_000, "1.00 TB"),
            (1000_000_000_000_00, "100.00 TB"),
            (1000_000_000_000_000, "1.00 PB"),
            (1000_000_000_000_000_00, "100.00 PB"),
            (1000_000_000_000_000_000, "1.00 EB"),
            (1000_000_000_000_000_000_0, "10.00 EB"),
        ];

        for (input, expected_output) in test_cases {
            let output = format_bytes(input);
            assert_eq!(output, expected_output);
        }
    }

    #[test]
    fn test_garbage_result_vec_cache() {
        let path = Path::new("/Users/testuser/Projects");
        let garbage_results = vec![
            GarbageRecognizerResult {
                index: GarbageIndex::Id(0),
                recognizer_name: "Rust".to_string(),
                directory: Default::default(),
                size: 0,
                deletable: vec![],
            },
            GarbageRecognizerResult {
                index: GarbageIndex::Id(1),
                recognizer_name: "Flutter".to_string(),
                directory: PathBuf::from("/Users/testuser/Projects/example"),
                size: 0,
                deletable: vec![PathBuf::from("/Users/testuser/Projects/example/target")],
            },
        ];

        let write_result = write_garbage_result_vec_cache(path, &garbage_results, None);
        assert!(write_result.is_ok());

        let read_result = read_garbage_result_vec_cache(path, None);
        assert!(write_result.is_ok());
    }

    #[test]
    fn test_is_cache_durable() {
        assert_eq!(
            is_cache_durable(SystemTime::now().add(Duration::from_secs(5))),
            true
        );
        assert_eq!(
            is_cache_durable(SystemTime::now().add(Duration::from_secs(10))),
            true
        );
        assert_eq!(
            is_cache_durable(SystemTime::now().sub(Duration::from_secs(5))),
            false
        );
        assert_eq!(
            is_cache_durable(SystemTime::now().sub(Duration::from_secs(10))),
            false
        );
    }

    #[test]
    fn test_delete_garbage_result_vec_cache() {
        let temp_dir = temp_dir().join("wsg");
        fs::create_dir_all(&temp_dir).expect("Failed to create temporary wsg_dev directory");

        let base64_file_name = generate_base64_from_path(Path::new("/Users/testuser/Projects"));
        let test_file_path = temp_dir.join(base64_file_name);
        fs::File::create(test_file_path).expect("Failed to create test file");

        let result = delete_garbage_result_vec_cache(Path::new("/Users/testuser/Projects"));

        assert!(result.is_ok());

        fs::remove_dir_all(&temp_dir).expect("Can't delete wsg_dev directory");
    }

    #[test]
    fn test_generate_base64_from_path() {
        assert_eq!(
            generate_base64_from_path(Path::new("/Users/testuser/Projects"),),
            "HU+HKWhsmqU"
        );
        assert_eq!(
            generate_base64_from_path(Path::new("/Users/testuser/Projects/"),),
            "HU+HKWhsmqU"
        );
        assert_eq!(
            generate_base64_from_path(Path::new("C:/Users/TestUser/Projects"),),
            "9sOUzqnWnG0"
        );
        assert_eq!(
            generate_base64_from_path(Path::new("C:/Users/TestUser/Projects/"),),
            "9sOUzqnWnG0"
        );
        assert_eq!(generate_base64_from_path(Path::new(""),), "vWCstljHnkU");
        assert_eq!(generate_base64_from_path(Path::new("/"),), "vWCstljHnkU");
    }
}
