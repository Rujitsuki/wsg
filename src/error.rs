
#[derive(Debug)]
pub enum GarbageError {
    IOError(std::io::Error),
    WalkdirError(walkdir::Error),
}

impl From<std::io::Error> for GarbageError {
    fn from(error: std::io::Error) -> Self {
        GarbageError::IOError(error)
    }
}

impl From<walkdir::Error> for GarbageError {
    fn from(error: walkdir::Error) -> Self {
        GarbageError::WalkdirError(error)
    }
}