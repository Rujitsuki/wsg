use std::fmt::{Debug, Display, Formatter};

#[derive(Debug)]
pub enum GarbageError {
    IOError(std::io::Error),
    WalkdirError(walkdir::Error),
    SerializationError(serde_json::Error),
}

impl Display for GarbageError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            GarbageError::IOError(error) => write!(f, "IOError: {}", error),
            GarbageError::WalkdirError(error) => write!(f, "Directory recursive error: {}", error),
            GarbageError::SerializationError(error) => write!(f, "Serialization error: {}", error),
        }
    }
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
    InvalidArgumentPath,
    IdNotExists(String),
    GarbageError(GarbageError),
}

impl Display for ApplicationError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match &self {
            ApplicationError::MissingArgumentPath => write!(f, "A path must be specified"),
            ApplicationError::InvalidArgumentPath => write!(f, "It must be a valid path"),
            ApplicationError::IdNotExists(id) => write!(f, "The id {} does not exists, please check if the id exists with --list", id),
            ApplicationError::GarbageError(error) => write!(f, "{}", error),
        }
    }
}

impl Debug for ApplicationError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.to_string())
    }
}

impl From<GarbageError> for ApplicationError {
    fn from(error: GarbageError) -> Self {
        ApplicationError::GarbageError(error)
    }
}