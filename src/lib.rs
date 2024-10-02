use std::{
    collections::{HashMap, HashSet},
    fs::{self, File},
    io::{self, BufReader},
    path::{Path, PathBuf},
};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use wax::Glob;

#[derive(PartialEq, Eq, Debug)]
pub enum CheckResult {
    Changed,
    Unchanged,
}

const CACHE_FILE_NAME: &str = ".mayberun";

#[derive(Serialize, Deserialize, Debug)]
struct GlobState {
    file_hashes: HashMap<PathBuf, String>,
    glob_results: HashMap<String, HashSet<PathBuf>>,
}

impl GlobState {
    fn load(path: &Path) -> Option<GlobState> {
        let file = File::open(path).ok()?;
        let reader = BufReader::new(file);
        let glob_state = serde_json::from_reader(reader).ok()?;
        glob_state
    }

    fn write(&self, filename: &Path) -> io::Result<()> {
        fs::write(
            filename,
            serde_json::to_string(self)
                .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?,
        )
    }
}

pub fn check_glob(root: &Path, glob: &str) -> io::Result<CheckResult> {
    match GlobState::load(&root.join(PathBuf::from(CACHE_FILE_NAME))) {
        None => Ok(CheckResult::Changed),
        Some(saved_result) => {
            let current_files = file_set(root, glob)?;

            if Some(&current_files) != saved_result.glob_results.get(glob) {
                Ok(CheckResult::Changed)
            } else {
                let any_file_changed = 'block: {
                    for path in current_files {
                        let saved_hash = saved_result.file_hashes.get(&path);
                        let current_hash = hash(&path)?;
                        if saved_hash != Some(&current_hash) {
                            break 'block true;
                        }
                    }
                    false
                };
                if any_file_changed {
                    Ok(CheckResult::Changed)
                } else {
                    Ok(CheckResult::Unchanged)
                }
            }
        }
    }
}

fn file_set(root: &Path, glob: &str) -> io::Result<HashSet<PathBuf>> {
    let glob =
        Glob::new(glob).map_err(|_e| io::Error::new(io::ErrorKind::InvalidInput, "Bad glob"))?;
    let mut result = HashSet::new();
    for entry in glob.walk(root) {
        result.insert(entry?.into_path());
    }
    Ok(result)
}

fn file_hashes(files: &HashSet<PathBuf>) -> io::Result<HashMap<PathBuf, String>> {
    let mut result = HashMap::new();

    for f in files {
        result.insert(f.to_owned(), hash(f)?);
    }

    Ok(result)
}

pub fn write_glob(cwd: &Path, output_glob: &str) -> io::Result<()> {
    let current_files = file_set(cwd, output_glob)?;
    let hashes = file_hashes(&current_files);
    let file_path = &cwd.join(PathBuf::from(CACHE_FILE_NAME));

    let to_save: GlobState = match GlobState::load(file_path) {
        None => {
            let mut glob_to_files: HashMap<String, HashSet<PathBuf>> = HashMap::new();
            glob_to_files.insert(output_glob.to_owned(), current_files);

            GlobState {
                glob_results: glob_to_files,
                file_hashes: hashes?,
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
                saved_result.file_hashes.insert(file, file_hash?);
            }

            saved_result
        }
    };
    to_save.write(file_path)?;
    Ok(())
}

fn hash(path: &Path) -> io::Result<String> {
    let file_contents = fs::read(path)?;
    let mut hasher: Sha256 = Sha256::new();
    hasher.update(file_contents);
    let result = hasher.finalize();
    Ok(hex::encode(result))
}

#[cfg(test)]
mod tests {
    use tempdir::{self, TempDir};

    use crate::*;
    #[test]
    fn no_initial_file() -> io::Result<()> {
        let test_dir = TempDir::new("tests").unwrap();
        println!("{:?}", test_dir.path());
        assert_eq!(
            CheckResult::Changed,
            check_glob(test_dir.path(), "**/*.txt")?
        );
        Ok(())
    }

    #[test]
    fn write_some_files() -> io::Result<()> {
        let test_dir = TempDir::new("tests")?;
        let path = &test_dir.path().join(PathBuf::from("test.txt"));
        fs::write(path, "Hello, World")?;

        assert_eq!(
            CheckResult::Changed,
            check_glob(test_dir.path(), "**/*.txt")?,
        );

        write_glob(test_dir.path(), "**/*.txt")?;
        assert_eq!(
            CheckResult::Unchanged,
            check_glob(test_dir.path(), "**/*.txt")?,
        );

        Ok(())
    }

    #[test]
    fn write_change_check() -> io::Result<()> {
        let test_dir = TempDir::new("tests")?;
        let path = &test_dir.path().join(PathBuf::from("test.txt"));

        fs::write(path, "Hello, World")?;
        write_glob(test_dir.path(), "**/*.txt")?;

        fs::write(path, "Hola, Mundo")?;
        let result = check_glob(test_dir.path(), "**/*.txt")?;

        assert_eq!(CheckResult::Changed, result);

        Ok(())
    }
}
