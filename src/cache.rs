use std::{
    collections::HashMap,
    fs,
    hash::Hash as _,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
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
        KeyHash(file::Xxhash(hasher.digest()))
    }
}

/// Calculate the number of days since January 1, 2000 (Unix epoch: 946684800)
fn days_since_year_2000() -> u16 {
    let epoch_2000 = 946_684_800_u64; // Unix timestamp for 2000-01-01 00:00:00 UTC
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    if now < epoch_2000 {
        return 0;
    }
    let secs_per_day = 24 * 60 * 60;
    let days = now.saturating_sub(epoch_2000) / secs_per_day;
    days.min(u16::MAX as u64) as u16
}

pub(crate) trait CacheWriter {
    fn done(&mut self, key: &Key);
    fn done_hash(&mut self, hash: KeyHash);
    fn flush(&mut self) -> Result<()>;
}

pub(crate) trait Cache: CacheWriter {
    fn needed(&self, key: &Key) -> bool;
}

impl CacheWriter for &mut dyn Cache {
    #[inline]
    fn done(&mut self, key: &Key) {
        (*self).done(key);
    }

    #[inline]
    fn done_hash(&mut self, hash: KeyHash) {
        (*self).done_hash(hash);
    }

    #[inline]
    fn flush(&mut self) -> Result<()> {
        (*self).flush()
    }
}

impl Cache for &mut dyn Cache {
    fn needed(&self, key: &Key) -> bool {
        (**self).needed(key)
    }
}

pub(crate) struct HashCache {
    hashes: HashMap<KeyHash, u16>,
    file: PathBuf,
}

// Header format: 2 bytes (major) + 2 bytes (minor) + 2 bytes (patch) = 8 bytes total
const HEADER_SIZE: usize = 6;
const RECORD_SIZE: usize = 10; // 2 bytes (u16) + 8 bytes (u64)

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
    pub(crate) fn new(file: PathBuf) -> Self {
        Self {
            hashes: HashMap::new(),
            file,
        }
    }

    fn filter(&mut self) -> usize {
        let current_days = days_since_year_2000();
        let cutoff_days = current_days.saturating_sub(30);
        let initial_count = self.hashes.len();
        self.hashes.retain(|_, &mut days| days >= cutoff_days);
        initial_count - self.hashes.len()
    }

    pub(crate) fn from_file(file: &Path) -> Result<Self> {
        let mut cache = Self::new(file.to_path_buf());
        if !file.exists() {
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
        assert_eq!(contents.len() % RECORD_SIZE, 0);
        self.hashes.reserve(contents.len() / RECORD_SIZE);
        #[allow(clippy::unwrap_used)]
        for chunk in contents.chunks_exact(RECORD_SIZE) {
            let days = u16::from_le_bytes(chunk[0..2].try_into().unwrap());
            let hash_value = u64::from_le_bytes(chunk[2..10].try_into().unwrap());
            self.hashes.insert(KeyHash(file::Xxhash(hash_value)), days);
        }
    }

    fn serialize(&mut self) -> Vec<u8> {
        let record_size = 8 + 2;
        // hash + date
        debug!(
            "Flushing cache of size {} to {}",
            self.hashes.len() * record_size,
            self.file.display(),
        );
        let filtered_count = self.filter();
        if filtered_count > 0 {
            debug!("Filtered {} hashes older than 30 days", filtered_count);
        }
        let mut entries: Vec<(u16, u64)> =
            self.hashes.iter().map(|(h, &days)| (days, h.0.0)).collect();
        entries.sort_by_key(|(_, hash)| *hash);
        let mut content = Vec::with_capacity(HEADER_SIZE + entries.len() * record_size);

        let (major, minor, patch) = current_version();
        content.extend_from_slice(&serialize_version_header(major, minor, patch));
        for (days, hash_value) in entries {
            content.extend_from_slice(&days.to_le_bytes());
            content.extend_from_slice(&hash_value.to_le_bytes());
        }
        content
    }
}

impl CacheWriter for HashCache {
    #[inline]
    fn done_hash(&mut self, hash: KeyHash) {
        let days = days_since_year_2000();
        self.hashes.insert(hash, days);
    }

    #[inline]
    fn done(&mut self, key: &Key) {
        self.done_hash(KeyHash::from(key));
    }

    fn flush(&mut self) -> Result<()> {
        let content = self.serialize();
        fs::write(&self.file, content)
            .with_context(|| format!("Failed to write cache file: {}", self.file.display()))?;
        Ok(())
    }
}

impl Cache for HashCache {
    #[inline]
    fn needed(&self, key: &Key) -> bool {
        !self.hashes.contains_key(&KeyHash::from(key))
    }
}

pub(crate) struct NopCache;

impl CacheWriter for NopCache {
    #[inline]
    fn done(&mut self, _key: &Key) {}

    #[inline]
    fn done_hash(&mut self, _hash: KeyHash) {}

    #[inline]
    fn flush(&mut self) -> Result<()> {
        Ok(())
    }
}

impl Cache for NopCache {
    #[inline]
    fn needed(&self, _key: &Key) -> bool {
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;

    fn create_test_key(path: &str, cmd: &str) -> Key {
        let mut hasher = Xxh3::new();
        hasher.update(cmd.as_bytes());
        let tool_stamp = tool::Stamp(file::Xxhash(hasher.digest()));
        Key::new(
            file::Stamp(file::Xxhash(path.len() as u64 + 12345)),
            tool_stamp,
        )
    }

    #[test]
    fn new_cache() {
        let temp_file = NamedTempFile::new().unwrap();
        let cache = HashCache::new(temp_file.path().to_path_buf());
        assert!(cache.hashes.is_empty());
        assert_eq!(cache.file, temp_file.path());
    }

    #[test]
    fn load_nonexistent_file() {
        let temp_file = NamedTempFile::new().unwrap();
        let file_path = temp_file.path().to_path_buf();
        drop(temp_file); // Delete the file

        let cache = HashCache::from_file(&file_path).unwrap();
        assert!(cache.hashes.is_empty());
    }

    #[test]
    fn load_empty_file() {
        let temp_file = NamedTempFile::new().unwrap();
        fs::write(temp_file.path(), b"").unwrap();

        let cache = HashCache::from_file(temp_file.path()).unwrap();
        assert!(cache.hashes.is_empty());
    }

    #[test]
    fn persistence() {
        let temp_file = NamedTempFile::new().unwrap();
        let key = create_test_key("test.rs", "cargo fmt");
        {
            let mut cache = HashCache::new(temp_file.path().to_path_buf());
            cache.done(&key);
            cache.flush().unwrap();
        }
        {
            let cache = HashCache::from_file(temp_file.path()).unwrap();
            assert!(!cache.needed(&key));
        }
    }
}
