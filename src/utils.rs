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