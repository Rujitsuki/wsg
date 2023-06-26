use std::fmt::{Debug, Display, Formatter};

#[derive(Debug)]
pub enum GarbageError {
    IOError(std::io::Error),
    WalkdirError(walkdir::Error),
    SerializationError(serde_json::Error),
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

impl From<serde_json::Error> for GarbageError {
    fn from(error: serde_json::Error) -> Self {
        GarbageError::SerializationError(error)
    }
}

pub enum ApplicationError {
    MissingArgumentPath,
    IdNotExists(String),
}

impl Display for ApplicationError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match &self {
            ApplicationError::MissingArgumentPath => write!(f, "A valid path must be specified"),
            ApplicationError::IdNotExists(id) => write!(f, "The id {} does not exists, please check if the id exists with --list", id),
        }
    }
}

impl Debug for ApplicationError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.to_string())
    }
}