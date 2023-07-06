use crate::error::{ApplicationError, GarbageError};
use crate::garbage::{
    clean_garbage_from_vec, compute_deletable_size_from_garbage_results, filter_garbage_from_ids,
    find_garbage_in_directory, GarbageIndex, GarbageRecognizer, GarbageRecognizerResult,
};
use crate::recognizer::available_recognizer;
use crate::ui::{BuildContext, Size, UIBox};
use crate::utils::{
    delete_all_cache_files, delete_garbage_result_vec_cache, format_bytes,
    read_garbage_result_vec_cache, write_garbage_result_vec_cache,
};
use clap::Parser;
use std::collections::HashSet;
use std::path::{Path, PathBuf};

mod error;
mod garbage;
mod recognizer;
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

    #[arg(long, value_name="RECOGNIZER", value_delimiter=',', num_args = 1.., help = "Start without any recognizer, only the selected ones are applied.")]
    include_recognizer: Option<Vec<String>>,

    #[arg(long, value_name="RECOGNIZER", value_delimiter=',', num_args = 1.., help = "Start with all available recognizers, only the elected are excluded.")]
    exclude_recognizer: Option<Vec<String>>,

    #[arg(long)]
    list_recognizer: bool,

    #[arg(long, help = "Clean the application cache for all listings")]
    clean_cache: bool,

    #[arg(long, help = "Force to renew the cache for specific path")]
    force: bool,
}

fn main() -> Result<(), ApplicationError> {
    let mut state = AppState::new();
    let args = Args::parse();

    register_garbage_recognizer(&mut state, &args);

    if args.clean_cache {
        delete_all_cache_files()?;
        println!("\nCache cleared successfully\n");
        return Ok(());
    }

    if args.list {
        let _path = match &args.path {
            None => Err(ApplicationError::MissingArgumentPath),
            Some(path) => Ok(path),
        }?;
        let _ = arg_list(&state, &_path, args.force);
        return Ok(());
    }

    if let Some(ids) = args.clean {
        let _path = match &args.path {
            None => Err(ApplicationError::MissingArgumentPath),
            Some(path) => Ok(path),
        }?;
        if let Err(_) = arg_clean(&_path, &ids) {
            let _ = arg_list(&state, _path, true);
            println!("\nYou should first get an overview before you delete anything!\nThe --clean command can now be used.\n");
        }
        return Ok(());
    }

    if args.path.is_some() && args.clean.is_none() && args.list == false {
        let _path = match args.path {
            None => Err(ApplicationError::InvalidArgumentPath),
            Some(path) => Ok(path),
        }?;
        let _ = arg_list(&state, &_path, args.force);
        return Ok(());
    }

    Ok(())
}

fn arg_list(state: &AppState, path: &Path, force: bool) -> Result<(), GarbageError> {
    let generate_garbage_result_without_cache =
        || -> Result<Vec<GarbageRecognizerResult>, GarbageError> {
            let garbage = find_garbage_in_directory(path, state)?;
            let _ = write_garbage_result_vec_cache(path, &garbage, None)?;
            Ok(garbage)
        };

    let result = if force {
        generate_garbage_result_without_cache()?
    } else {
        match read_garbage_result_vec_cache(path, None) {
            Ok(vec) => vec,
            Err(_) => generate_garbage_result_without_cache()?,
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
    let garbage = read_garbage_result_vec_cache(path, None)?;
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

fn register_garbage_recognizer(state: &mut AppState, args: &Args) {
    let mut recognizer = available_recognizer();

    include_recognizer(&mut recognizer, args);
    exclude_recognizer(&mut recognizer, args);

    state.garbage_recognizer.extend(recognizer);
}

fn include_recognizer(recognizer_vec: &mut Vec<GarbageRecognizer>, args: &Args) {
    if let Some(include_recognizer) = &args.include_recognizer {
        recognizer_vec.retain(|r| include_recognizer.contains(&r.name.to_lowercase()));
    }
}

fn exclude_recognizer(recognizer_vec: &mut Vec<GarbageRecognizer>, args: &Args) {
    if let Some(exclude_recognizer) = &args.exclude_recognizer {
        recognizer_vec.retain(|r| !exclude_recognizer.contains(&r.name.to_lowercase()));
    }
}
