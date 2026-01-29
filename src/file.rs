use anyhow::{Context, Result};
use std::fs;
use std::path::{Path, PathBuf};
use xxhash_rust::xxh3::Xxh3;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) struct Xxhash(pub(crate) u128);

/// Hash of file path, content, and metadata
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) struct Stamp(pub(crate) Xxhash);

#[derive(Clone, Debug)]
pub(crate) struct File {
    pub(crate) path: PathBuf,
    pub(crate) size: usize,
    pub(crate) metadata_stamp: Stamp,
    pub(crate) mtime_stamp: Stamp,
    pub(crate) content_stamp: Option<Stamp>,
}

pub(crate) fn hash_md(path: &Path, metadata: &fs::Metadata, md: &mut Xxh3) {
    md.update(path.as_os_str().as_encoded_bytes());
    md.update(&metadata.len().to_le_bytes());
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        md.update(&metadata.uid().to_le_bytes());
        md.update(&metadata.gid().to_le_bytes());
        md.update(&metadata.mode().to_le_bytes());
    }
}

fn compute_md_stamp(path: &Path, metadata: &fs::Metadata) -> Stamp {
    let mut md = Xxh3::new();
    hash_md(path, metadata, &mut md);
    Stamp(Xxhash(md.digest128()))
}

pub(crate) fn hash_mtime(
    path: &Path,
    metadata: &fs::Metadata,
    mtime_hasher: &mut Xxh3,
) -> Result<(), anyhow::Error> {
    let mtime = metadata
        .modified()
        .with_context(|| format!("Failed to get modification time for: {}", path.display()))?;
    mtime_hasher.update(
        &mtime
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos()
            .to_le_bytes(),
    );
    Ok(())
}

fn compute_mtime_stamp(path: &Path, metadata: &fs::Metadata) -> Result<Stamp, anyhow::Error> {
    let mut mtime_hasher = Xxh3::new();
    hash_mtime(path, metadata, &mut mtime_hasher)?;
    let mtime_stamp = Stamp(Xxhash(mtime_hasher.digest128()));
    Ok(mtime_stamp)
}

impl File {
    pub(crate) fn new(path: PathBuf) -> Result<Self> {
        let metadata = fs::metadata(&path)
            .with_context(|| format!("Failed to get metadata for: {}", path.display()))?;
        let metadata_stamp = compute_md_stamp(&path, &metadata);
        let mtime_stamp = compute_mtime_stamp(&path, &metadata)?;
        let size = usize::try_from(metadata.len())
            .with_context(|| format!("File too bug: {}b", metadata.len()))?;
        Ok(Self {
            path,
            size,
            metadata_stamp,
            mtime_stamp,
            content_stamp: None,
        })
    }

    /// Fill in the content stamp by reading the file content
    pub(crate) fn fill_content_stamp(&mut self) -> Result<()> {
        if self.content_stamp.is_some() {
            return Ok(());
        }
        let content = fs::read(&self.path)
            .with_context(|| format!("Failed to read file: {}", self.path.display()))?;
        self.content_stamp = Some(Stamp(compute_hash(&content)));
        Ok(())
    }

    pub(crate) fn content_stamp(&self) -> Stamp {
        debug_assert!(self.content_stamp.is_some());
        let mut hasher = Xxh3::new();
        hasher.update(&self.metadata_stamp.0.0.to_le_bytes());
        if let Some(content_stamp) = self.content_stamp {
            hasher.update(&content_stamp.0.0.to_le_bytes());
        }
        Stamp(Xxhash(hasher.digest128()))
    }

    pub(crate) fn mtime_stamp(&self) -> Stamp {
        let mut hasher = Xxh3::new();
        hasher.update(&self.metadata_stamp.0.0.to_le_bytes());
        hasher.update(&self.mtime_stamp.0.0.to_le_bytes());
        Stamp(Xxhash(hasher.digest128()))
    }
}

pub(crate) fn compute_hash(content: &[u8]) -> Xxhash {
    let mut hasher = Xxh3::new();
    hasher.update(content);
    Xxhash(hasher.digest128())
}
