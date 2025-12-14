use crate::{LUN_VERSION, file, run::RunMode};
use anyhow::{Context, Result};
use std::{
    collections::HashMap,
    fs,
    hash::Hash as _,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};
use tracing::{debug, warn};
use xxhash_rust::xxh3::Xxh3;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) struct Key {
    pub(crate) stamp: file::Stamp,
    pub(crate) cmd: file::Xxhash,
    pub(crate) config_file_content: Option<file::Xxhash>,
    pub(crate) tool_version: Option<file::Xxhash>,
}

impl Key {
    #[cfg(test)]
    pub(crate) fn new(
        stamp: file::Stamp,
        cmd: &str,
        config_file_content: Option<file::Xxhash>,
        tool_version: Option<file::Xxhash>,
    ) -> Self {
        Self {
            stamp,
            cmd: file::compute_hash(cmd.as_bytes()),
            config_file_content,
            tool_version,
        }
    }

    pub(crate) fn from_file_and_tool(
        file: &file::File,
        tool: &crate::tool::Tool,
        mode: RunMode,
    ) -> Self {
        Self {
            stamp: file.stamp,
            cmd: file::compute_hash(tool.get_cmd(mode).as_bytes()),
            config_file_content: tool.config,
            tool_version: tool.version,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) struct KeyHash(pub(crate) file::Xxhash);

impl From<&Key> for KeyHash {
    fn from(key: &Key) -> Self {
        let mut hasher = Xxh3::new();
        hasher.update(LUN_VERSION.as_bytes());
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

pub(crate) struct HashCache {
    hashes: HashMap<KeyHash, u16>,
    file: PathBuf,
}

const CACHE_VERSION: u32 = 1;
const HEADER_SIZE: usize = size_of_val(&CACHE_VERSION);
const RECORD_SIZE: usize = 10; // 2 bytes (u16) + 8 bytes (u64)

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

        #[allow(clippy::unwrap_used)]
        let version = u32::from_le_bytes(contents[0..HEADER_SIZE].try_into().unwrap());
        if version != CACHE_VERSION {
            warn!(
                "Cache version mismatch at {} (expected: {}, found: {})",
                file.display(),
                CACHE_VERSION,
                version,
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

        content.extend_from_slice(&CACHE_VERSION.to_le_bytes());
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

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;

    fn create_test_key(path: &str, cmd: &str) -> Key {
        Key::new(
            file::Stamp(file::Xxhash(path.len() as u64 + 12345)),
            cmd,
            None,
            None,
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
