use crate::garbage::{FileType, GarbageRecognizer};

pub fn available_recognizer() -> Vec<GarbageRecognizer> {
    vec![
        GarbageRecognizer::new(
            "Flutter",
            Some(vec![FileType::File("pubspec.yaml".into())]),
            Some(vec![FileType::Directory("build".into())]),
        ),
        GarbageRecognizer::new(
            "NodeJS",
            Some(vec![FileType::File("package.json".into())]),
            Some(vec![FileType::Directory("node_modules".into())]),
        ),
        GarbageRecognizer::new(
            "Rust",
            Some(vec![FileType::File("Cargo.toml".into())]),
            Some(vec![FileType::Directory("target".into())]),
        ),
    ]
}
