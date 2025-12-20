use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use xxhash_rust::xxh3::Xxh3;

use crate::cache::{CacheWriter, HashCache, Key, KeyHash};
use crate::file;
use crate::tool;

pub(crate) fn add(cache_file: &Path, string: &str, files: &[PathBuf]) -> Result<(), anyhow::Error> {
    let mut hasher = Xxh3::new();
    hasher.update(string.as_bytes());
    let tool_stamp = tool::Stamp(file::Xxhash(hasher.digest()));
    let mut cache = HashCache::from_file(cache_file, None)?;
    for file_path in files {
        let file = file::File::new(file_path.clone())
            .with_context(|| format!("Failed to read file: {}", file_path.display()))?;
        let key = Key {
            stamp: file.mtime_stamp(),
            tool_stamp,
        };
        cache.done(&key);
    }
    cache.flush()?;
    Ok(())
}

pub(crate) fn get(
    cache_file: &Path,
    string: &str,
    files: &[PathBuf],
    null_separated: bool,
) -> Result<(), anyhow::Error> {
    let mut hasher = Xxh3::new();
    hasher.update(string.as_bytes());
    let tool_stamp = tool::Stamp(file::Xxhash(hasher.digest()));
    let cache = HashCache::from_file(cache_file, None)?;
    for file_path in files {
        let file = file::File::new(file_path.clone())
            .with_context(|| format!("Failed to read file: {}", file_path.display()))?;
        let key = Key {
            stamp: file.mtime_stamp(),
            tool_stamp,
        };
        let hash = KeyHash::from(&key);
        let found = cache.hashes.contains_key(&hash);
        if null_separated {
            print!("{found}\0");
        } else {
            println!("{found}");
        }
    }
    Ok(())
}

pub(crate) fn rm(cache_file: &Path, string: &str, files: &[PathBuf]) -> Result<(), anyhow::Error> {
    let mut hasher = Xxh3::new();
    hasher.update(string.as_bytes());
    let tool_stamp = tool::Stamp(file::Xxhash(hasher.digest()));
    let mut cache = HashCache::from_file(cache_file, None)?;
    for file_path in files {
        let file = file::File::new(file_path.clone())
            .with_context(|| format!("Failed to read file: {}", file_path.display()))?;
        let key = Key {
            stamp: file.mtime_stamp(),
            tool_stamp,
        };
        let hash = KeyHash::from(&key);
        let was_present = cache.hashes.remove(&hash).is_some();
        println!("{was_present}");
    }
    cache.flush()?;
    Ok(())
}
