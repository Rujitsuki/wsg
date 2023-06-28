use crate::error::{ApplicationError, GarbageError};
use crate::garbage::{
    clean_garbage_from_vec, compute_deletable_size_from_garbage_results, filter_garbage_from_ids,
    find_garbage_in_directory, FileType, GarbageIndex, GarbageRecognizer, GarbageRecognizerResult,
};
use crate::ui::{BuildContext, Size, UIBox};
use crate::utils::{
    delete_garbage_result_vec_cache, format_bytes, read_garbage_result_vec_cache,
    write_garbage_result_vec_cache,
};
use clap::Parser;
use std::collections::HashSet;
use std::env;
use std::path::{Path, PathBuf};

mod error;
mod garbage;
mod ui;
mod utils;

pub struct AppState {
    garbage_recognizer: HashSet<GarbageRecognizer>,
}

impl AppState {
    pub fn new() -> Self {
        AppState {
            garbage_recognizer: HashSet::new(),
        }
    }

    pub fn register_garbage_recognizer(&mut self, recognizer: GarbageRecognizer) {
        self.garbage_recognizer.insert(recognizer);
    }
}

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    path: Option<PathBuf>,

    #[arg(short, long, help = "List all the garbage in directory")]
    list: bool,

    #[arg(short, long, value_delimiter = ',', num_args = 1.., value_name = "index", help = "Delete all the garbage in directory")]
    clean: Option<Vec<GarbageIndex>>,

    #[arg(long, value_name="RECOGNIZER", value_delimiter=',', num_args = 1.., help = "")]
    include_recognizer: Option<Vec<String>>,

    #[arg(long, value_name="RECOGNIZER", value_delimiter=',', num_args = 1..)]
    exclude_recognizer: Option<Vec<String>>,

    #[arg(long)]
    list_recognizer: bool,
}

fn main() -> Result<(), ApplicationError> {
    let mut state = AppState::new();
    let args = Args::parse();

    register_garbage_recognizer(&mut state);

    if args.path.is_some() && args.clean.is_none() && args.list == false {
        let _path = match env::current_dir() {
            Err(_) => Err(ApplicationError::InvalidArgumentPath),
            Ok(path) => Ok(path),
        }?;
        let _ = arg_list(&state, &_path);
    }

    if args.list {
        let _path = match &args.path {
            None => Err(ApplicationError::MissingArgumentPath),
            Some(path) => Ok(path),
        }?;
        let _ = arg_list(&state, &_path);
    }

    if let Some(ids) = args.clean {
        let _path = match &args.path {
            None => Err(ApplicationError::MissingArgumentPath),
            Some(path) => Ok(path),
        }?;
        arg_clean(&_path, &ids).unwrap();
    }

    Ok(())
}

fn arg_list(state: &AppState, path: &Path) -> Result<(), GarbageError> {
    let result = match read_garbage_result_vec_cache(path) {
        Ok(vec) => vec,
        Err(_) => {
            let garbage = find_garbage_in_directory(path, state)?;
            let _ = write_garbage_result_vec_cache(path, &garbage)?;
            garbage
        }
    };

    display_garbage_results(&result)?;

    Ok(())
}

fn display_garbage_results(results: &Vec<GarbageRecognizerResult>) -> Result<(), GarbageError> {
    let terminal_size = crossterm::terminal::size()?;
    let context = BuildContext::new(Size::new(
        terminal_size.0 as usize,
        terminal_size.1 as usize,
    ));

    results.iter().for_each(|entry| {
        println!();
        let entry_string = format!(
            "Project folder: {:?}\nto clean: {}\nDeletable {:?}",
            entry.directory,
            format_bytes(entry.size),
            entry.deletable
        );
        let entry_box = UIBox::new(
            &context,
            format!(" [{}] {} ", entry.index, entry.recognizer_name),
            entry_string,
        );
        entry_box.render();
        println!();
    });

    println!(
        "Cleanable storage: {}\n",
        format_bytes(compute_deletable_size_from_garbage_results(results))
    );

    println!("Use the --clean <ids...> argument to clear the garbage. <ids...> can be 'all' or integers separated by a comma eg. 1,2,7");

    Ok(())
}

fn arg_clean(path: &Path, ids: &Vec<GarbageIndex>) -> Result<(), GarbageError> {
    let garbage = read_garbage_result_vec_cache(path)?;
    let filtered_garbage = filter_garbage_from_ids(garbage, &ids);

    display_garbage_to_clean(&filtered_garbage);

    println!("Are you sure you want to delete the files listed above? (y/N):");

    let mut input = String::new();
    std::io::stdin().read_line(&mut input)?;

    let confirmation = input.trim().eq_ignore_ascii_case("y");

    if confirmation {
        clean_garbage_from_vec(filtered_garbage)?;
        delete_garbage_result_vec_cache(path)?;
        println!("The garbage has been deleted successfully!");
    }

    Ok(())
}

fn display_garbage_to_clean(results: &Vec<GarbageRecognizerResult>) {
    results.iter().for_each(|garbage| {
        println!("[{}] - {}", garbage.index, garbage.directory.display());
        println!(
            "\t{}, to clean: {}",
            garbage.recognizer_name,
            format_bytes(garbage.size)
        );
        for deletable_path in &garbage.deletable {
            println!("\tDelete: {}", deletable_path.display())
        }
        println!();
    });
    println!();
}

fn register_garbage_recognizer(state: &mut AppState) {
    state.register_garbage_recognizer(GarbageRecognizer::new(
        "Flutter",
        Some(vec![FileType::File("pubspec.yaml".into())]),
        Some(vec![FileType::Directory("build".into())]),
    ));
    state.register_garbage_recognizer(GarbageRecognizer::new(
        "NodeJS",
        Some(vec![FileType::File("package.json".into())]),
        Some(vec![FileType::Directory("node_modules".into())]),
    ));
    state.register_garbage_recognizer(GarbageRecognizer::new(
        "Rust, Cargo",
        Some(vec![FileType::File("Cargo.toml".into())]),
        Some(vec![FileType::Directory("target".into())]),
    ));
}
