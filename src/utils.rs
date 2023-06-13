use std::fs;
use std::path::PathBuf;

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