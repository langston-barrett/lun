use std::{
    collections::HashMap,
    fs,
    hash::Hash as _,
    mem::size_of,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result};
use tracing::{debug, info, warn};
use xxhash_rust::xxh3::Xxh3;

use crate::file;
use crate::tool;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) struct Key {
    pub(crate) stamp: file::Stamp,
    pub(crate) tool_stamp: tool::Stamp,
}

impl Key {
    #[cfg(test)]
    pub(crate) fn new(stamp: file::Stamp, tool_stamp: tool::Stamp) -> Self {
        Self { stamp, tool_stamp }
    }

    pub(crate) fn from_content(file: &file::File, tool: &tool::Tool) -> Self {
        Self {
            stamp: file.content_stamp(),
            tool_stamp: tool.stamp,
        }
    }

    pub(crate) fn from_mtime(file: &file::File, tool: &tool::Tool) -> Self {
        Self {
            stamp: file.mtime_stamp(),
            tool_stamp: tool.stamp,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) struct KeyHash(pub(crate) file::Xxhash);

impl From<&Key> for KeyHash {
    fn from(key: &Key) -> Self {
        let mut hasher = Xxh3::new();
        key.hash(&mut hasher);
        KeyHash(file::Xxhash(hasher.digest128()))
    }
}

pub(crate) trait CacheWriter {
    fn done(&mut self, key: &Key);
    fn done_hash(&mut self, hash: KeyHash);
    fn flush(&mut self) -> Result<bool>;
}

pub(crate) trait Cache: CacheWriter {
    fn needed(&mut self, key: &Key) -> bool;
}

pub(crate) struct HashCache {
    pub(crate) hashes: HashMap<KeyHash, u16>,
    file: PathBuf,
    pub(crate) max_entries: usize,
    pub(crate) entries_added: usize, // used in warnings
}

// Header format: 2 bytes (major) + 2 bytes (minor) + 2 bytes (patch) = 6 bytes total
const HEADER_SIZE: usize = 6;
const RECORD_SIZE: usize = size_of::<u16>() + size_of::<KeyHash>(); // 2 bytes (u16 counter) + 16 bytes (u128 hash)
// For reference rust-lang/rust has 32000 (~ 2^15) .rs files
// 2^17 * 18 bytes is ~ 2.25 MiB
pub(crate) const DEFAULT_MAX_CACHE_SIZE_BYTES: usize = (2 << 17) * RECORD_SIZE;

/// Calculate the maximum number of cache entries from a byte size.
/// Rounds down to the next lowest multiple of RECORD_SIZE.
pub(crate) fn max_entries_from_bytes(bytes: usize) -> usize {
    bytes / RECORD_SIZE
}

#[allow(clippy::unwrap_used)]
fn current_version() -> (u16, u16, u16) {
    (
        const { u16::from_str_radix(env!("CARGO_PKG_VERSION_MAJOR"), 10) }.unwrap(),
        const { u16::from_str_radix(env!("CARGO_PKG_VERSION_MINOR"), 10) }.unwrap(),
        const { u16::from_str_radix(env!("CARGO_PKG_VERSION_PATCH"), 10) }.unwrap(),
    )
}

fn serialize_version_header(major: u16, minor: u16, patch: u16) -> [u8; HEADER_SIZE] {
    let mut header = [0u8; HEADER_SIZE];
    header[0..2].copy_from_slice(&major.to_le_bytes());
    header[2..4].copy_from_slice(&minor.to_le_bytes());
    header[4..6].copy_from_slice(&patch.to_le_bytes());
    header
}

fn deserialize_version_header(header: &[u8]) -> Option<(u16, u16, u16)> {
    if header.len() < HEADER_SIZE {
        return None;
    }
    let major_bytes: [u8; 2] = header[0..2].try_into().ok()?;
    let minor_bytes: [u8; 2] = header[2..4].try_into().ok()?;
    let patch_bytes: [u8; 2] = header[4..6].try_into().ok()?;
    let major = u16::from_le_bytes(major_bytes);
    let minor = u16::from_le_bytes(minor_bytes);
    let patch = u16::from_le_bytes(patch_bytes);
    Some((major, minor, patch))
}

impl HashCache {
    #[inline]
    pub(crate) fn new(file: PathBuf, max_size_entries: usize) -> Self {
        Self {
            hashes: HashMap::new(),
            file,
            max_entries: max_size_entries,
            entries_added: 0,
        }
    }

    pub(crate) fn from_file(file: &Path, max_size_bytes: Option<usize>) -> Result<Self> {
        let max_size_entries = max_size_bytes.map_or_else(
            || max_entries_from_bytes(DEFAULT_MAX_CACHE_SIZE_BYTES),
            max_entries_from_bytes,
        );
        let mut cache = Self::new(file.to_path_buf(), max_size_entries);
        if !file.exists() {
            debug!("No cache at {}", file.display());
            return Ok(cache);
        }
        cache.load(file)?;
        Ok(cache)
    }

    fn cache_ok(file: &Path, contents: &[u8]) -> bool {
        if contents.len() < HEADER_SIZE {
            warn!(
                "Corrupted cache at {} (size: {})",
                file.display(),
                contents.len(),
            );
            return false;
        }

        let Some((cached_major, cached_minor, cached_patch)) =
            deserialize_version_header(&contents[0..HEADER_SIZE])
        else {
            warn!("Corrupted cache header at {}", file.display(),);
            return false;
        };

        let (current_major, current_minor, current_patch) = current_version();
        if (cached_major, cached_minor, cached_patch)
            != (current_major, current_minor, current_patch)
        {
            info!(
                "Cache version mismatch at {} (lun: {}.{}.{}, cache: {}.{}.{})",
                file.display(),
                current_major,
                current_minor,
                current_patch,
                cached_major,
                cached_minor,
                cached_patch,
            );
            return false;
        }

        if !(contents.len() - HEADER_SIZE).is_multiple_of(RECORD_SIZE) {
            warn!(
                "Corrupted cache at {} (size: {})",
                file.display(),
                contents.len(),
            );
            return false;
        }

        true
    }

    fn load(&mut self, file: &Path) -> Result<(), anyhow::Error> {
        debug!("Loading cache from {}", file.display());
        let contents = fs::read(file)
            .with_context(|| format!("Failed to read cache file: {}", file.display()))?;
        if !Self::cache_ok(file, &contents) {
            drop(fs::remove_file(file));
            return Ok(());
        }
        let records_data = &contents[HEADER_SIZE..];
        self.load_records(records_data);
        debug!("Loaded {} hashes", self.hashes.len());
        Ok(())
    }

    fn load_records(&mut self, contents: &[u8]) {
        assert_eq!(contents.len() % RECORD_SIZE, 0); // cache_ok
        self.hashes.reserve(contents.len() / RECORD_SIZE);
        #[allow(clippy::unwrap_used)]
        for chunk in contents.chunks_exact(RECORD_SIZE) {
            let counter = u16::from_le_bytes(chunk[0..size_of::<u16>()].try_into().unwrap());
            let hash_value = u128::from_le_bytes(
                chunk[size_of::<u16>()..size_of::<u16>() + size_of::<KeyHash>()]
                    .try_into()
                    .unwrap(),
            );
            self.hashes
                .insert(KeyHash(file::Xxhash(hash_value)), counter);
        }
    }

    fn serialize(&mut self) -> (Vec<u8>, bool) {
        debug!(
            "Flushing cache of size {} to {}",
            self.hashes.len() * RECORD_SIZE,
            self.file.display(),
        );

        let mut entries: Vec<(u16, u128)> = self
            .hashes
            .iter()
            .map(|(h, &counter)| (counter.saturating_add(1), h.0.0))
            .collect();

        // Sort by counter, then by hash
        entries.sort_by(|a, b| a.0.cmp(&b.0).then_with(|| a.1.cmp(&b.1)));
        let initial_count = entries.len();
        let to_keep = entries.len().min(self.max_entries);
        let removed_count = initial_count.saturating_sub(to_keep);
        let cache_full = removed_count > 0;
        debug!("Dropping {} old cache entries", removed_count);

        let mut content = Vec::with_capacity(HEADER_SIZE + to_keep * RECORD_SIZE);
        let (major, minor, patch) = current_version();
        content.extend_from_slice(&serialize_version_header(major, minor, patch));

        for (counter, hash_value) in entries.into_iter().take(to_keep) {
            debug_assert_eq!(
                counter.to_le_bytes().len() + hash_value.to_le_bytes().len(),
                RECORD_SIZE
            );
            content.extend_from_slice(&counter.to_le_bytes());
            content.extend_from_slice(&hash_value.to_le_bytes());
        }
        (content, cache_full)
    }
}

impl CacheWriter for HashCache {
    #[inline]
    fn done_hash(&mut self, hash: KeyHash) {
        let was_new = self.hashes.insert(hash, 0).is_none();
        debug_assert!(was_new);
        self.entries_added += 1;
    }

    #[inline]
    fn done(&mut self, key: &Key) {
        debug_assert!(self.needed(key));
        self.done_hash(KeyHash::from(key));
    }

    fn flush(&mut self) -> Result<bool> {
        let (content, cache_full) = self.serialize();
        fs::write(&self.file, content)
            .with_context(|| format!("Failed to write cache file: {}", self.file.display()))?;
        Ok(cache_full)
    }
}

impl Cache for HashCache {
    #[inline]
    fn needed(&mut self, key: &Key) -> bool {
        let hash = KeyHash::from(key);
        self.hashes.entry(hash).and_modify(|e| *e = 0);
        !self.hashes.contains_key(&hash)
    }
}

pub(crate) fn rm(path: &Path) -> Result<(), anyhow::Error> {
    if path.exists() {
        fs::remove_dir_all(path)
            .with_context(|| format!("Failed to remove cache: {}", path.display()))?;
        debug!("Cache removed from {}", path.display());
    }
    Ok(())
}

pub(crate) fn gc(cache_file: &Path, max_size_bytes: Option<usize>) -> Result<(), anyhow::Error> {
    if !cache_file.exists() {
        info!("No cache file at {}", cache_file.display());
        return Ok(());
    }
    let max_size_bytes = max_size_bytes.unwrap_or(DEFAULT_MAX_CACHE_SIZE_BYTES);
    let mut cache = HashCache::from_file(cache_file, Some(max_size_bytes))?;
    let cache_full = cache.flush()?;
    if cache_full {
        info!("Cache reduced to {} bytes", max_size_bytes);
    } else {
        info!("Cache already within size limit");
    }
    Ok(())
}

pub(crate) fn stats(cache_file: &Path) -> Result<(), anyhow::Error> {
    const KIBI: usize = 1024;
    const TWO_KIBI: usize = 2 * KIBI;
    if !cache_file.exists() {
        info!("No cache file at {}", cache_file.display());
        return Ok(());
    }
    let cache = HashCache::from_file(cache_file, None)?;
    let records = cache.hashes.len();
    let total_size_bytes = HEADER_SIZE + records * RECORD_SIZE;
    let max_records = cache.max_entries;
    let max_size_bytes = HEADER_SIZE + max_records * RECORD_SIZE;
    let percentage_used = if max_records > 0 {
        (records * 100) / max_records
    } else {
        0
    };

    // Calculate records in most recent run (counter == 0)
    let records_most_recent_run = cache
        .hashes
        .values()
        .filter(|&&counter| counter == 0)
        .count();

    // Calculate average records per run
    // Count records by counter value to determine how many runs are represented
    let max_counter = cache.hashes.values().max().copied().unwrap_or(0);
    let runs_represented = if records > 0 {
        max_counter as usize + 1
    } else {
        0
    };
    let avg_records_per_run = if runs_represented > 0 {
        records / runs_represented
    } else {
        0
    };

    info!("Number of runs: {runs_represented}");
    info!("Records: {records}");
    if total_size_bytes > TWO_KIBI {
        let total_size_kibi = total_size_bytes / KIBI;
        info!("Total size: {total_size_kibi} KiB");
    } else {
        info!("Total size: {total_size_bytes} bytes");
    }
    info!("Max records: {max_records}");
    if max_size_bytes > TWO_KIBI {
        let max_size_kibi = max_size_bytes / KIBI;
        info!("Max size: {max_size_kibi} KiB");
    } else {
        info!("Max size: {max_size_bytes} bytes");
    }
    info!("Percentage used: {percentage_used}%");
    info!("Records in most recent run: {records_most_recent_run}");
    info!("Average records per run: {avg_records_per_run}");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;

    fn create_test_key(path: &str, cmd: &str) -> Key {
        let mut hasher = Xxh3::new();
        hasher.update(cmd.as_bytes());
        let tool_stamp = tool::Stamp(file::Xxhash(hasher.digest128()));
        Key::new(
            file::Stamp(file::Xxhash(path.len() as u128 + 12345)),
            tool_stamp,
        )
    }

    #[test]
    fn new_cache() {
        let temp_file = NamedTempFile::new().unwrap();
        let cache = HashCache::new(temp_file.path().to_path_buf(), 1000);
        assert!(cache.hashes.is_empty());
        assert_eq!(cache.file, temp_file.path());
    }

    #[test]
    fn load_nonexistent_file() {
        let temp_file = NamedTempFile::new().unwrap();
        let file_path = temp_file.path().to_path_buf();
        drop(temp_file); // Delete the file

        let cache = HashCache::from_file(&file_path, None).unwrap();
        assert!(cache.hashes.is_empty());
    }

    #[test]
    fn load_empty_file() {
        let temp_file = NamedTempFile::new().unwrap();
        fs::write(temp_file.path(), b"").unwrap();

        let cache = HashCache::from_file(temp_file.path(), None).unwrap();
        assert!(cache.hashes.is_empty());
    }

    #[test]
    fn persistence() {
        let temp_file = NamedTempFile::new().unwrap();
        let key = create_test_key("test.rs", "cargo fmt");
        {
            let mut cache = HashCache::new(temp_file.path().to_path_buf(), 1000);
            cache.done(&key);
            cache.flush().unwrap();
        }
        {
            let mut cache = HashCache::from_file(temp_file.path(), None).unwrap();
            assert!(!cache.needed(&key));
        }
    }
}
