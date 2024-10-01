use std::{
    collections::{HashMap, HashSet},
    fs::{self, File},
    io::BufReader,
};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

#[derive(PartialEq, Eq)]
pub enum CheckResult {
    Changed,
    Unchanged,
}

#[derive(Serialize, Deserialize, Debug)]
struct GlobState {
    file_hashes: HashMap<String, String>,
    glob_results: HashMap<String, HashSet<String>>,
}

impl GlobState {
    fn paths(&self, glob: &str) -> HashSet<String> {
        self.glob_results.keys().map(|k| k.to_owned()).collect()
    }

    fn load(filename: &str) -> Option<GlobState> {
        let file = File::open(filename).ok()?;
        let reader = BufReader::new(file);
        let glob_state = serde_json::from_reader(reader).ok()?;
        glob_state
    }

    fn write(&self, filename: &str) {
        fs::write(
            filename,
            serde_json::to_string(self).expect("Failed to write"),
        );
    }
}

pub fn check_glob(input_glob: &str) -> CheckResult {
    match GlobState::load(".mayberun") {
        None => CheckResult::Changed,
        Some(saved_result) => {
            let current_files = file_set(input_glob);

            if current_files != saved_result.paths(input_glob) {
                CheckResult::Changed
            } else if current_files
                .into_iter()
                .any(|path| Some(&hash(&path)) != saved_result.file_hashes.get(&path))
            {
                CheckResult::Changed
            } else {
                CheckResult::Unchanged
            }
        }
    }
}

fn file_set(input_glob: &str) -> HashSet<String> {
    glob::glob(input_glob)
        .expect("Failed")
        .into_iter()
        .map(|el| el.expect("failed").as_os_str().to_str().unwrap().to_owned())
        .collect()
}

fn file_hashes(files: &HashSet<String>) -> HashMap<String, String> {
    let mut result = HashMap::new();

    for f in files {
        result.insert(f.to_owned(), hash(f));
    }

    result
}

fn write_glob(output_glob: &str) {
    let current_files = file_set(output_glob);
    let hashes = file_hashes(&current_files);

    let to_save: GlobState = match GlobState::load(".mayberun") {
        None => {
            let mut glob_to_files: HashMap<String, HashSet<String>> = HashMap::new();
            glob_to_files.insert(output_glob.to_owned(), current_files);

            GlobState {
                glob_results: glob_to_files,
                file_hashes: hashes,
            }
        }
        Some(mut saved_result) => {
            let saved_files_for_glob = saved_result.glob_results.get(output_glob);
            if Some(&current_files) != saved_files_for_glob {
                saved_result
                    .glob_results
                    .insert(output_glob.to_owned(), current_files.clone());
            }

            for file in current_files {
                let file_hash = hash(&file);
                saved_result.file_hashes.insert(file, file_hash);
            }

            saved_result
        }
    };
    to_save.write(".mayberun");
}

fn hash(path: &str) -> String {
    let file_contents = fs::read(path).unwrap();
    let mut hasher: Sha256 = Sha256::new();
    hasher.update(file_contents);
    let result = hasher.finalize();
    hex::encode(result)
}
