//! The in-memory registry of files the user has explicitly shared.
//!
//! Files are *referenced*, never copied: a 50 GB video costs a struct here and
//! is streamed from wherever it already lives. The registry is the only place
//! that maps a public, opaque id to a path on disk.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::RwLock;
use std::time::{SystemTime, UNIX_EPOCH};

use serde::Serialize;

use crate::error::{Error, Result};
use crate::security::paths;
use crate::security::tokens;

/// Upper bound on files pulled in from a single dropped folder, so dropping a
/// home directory cannot lock up the UI or exhaust memory.
pub const MAX_FILES_PER_FOLDER: usize = 2_000;

/// How deep a dropped folder is walked.
pub const MAX_FOLDER_DEPTH: usize = 12;

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ShareItem {
    /// Opaque, unguessable public identifier. This is what appears in URLs.
    pub id: String,
    /// What the user and the browser page see.
    pub display_name: String,
    /// Canonical absolute path. Never leaves the Rust side.
    #[serde(skip)]
    pub absolute_path: PathBuf,
    pub mime_type: String,
    pub size: u64,
    /// Unix milliseconds.
    pub added_at: u64,
    /// Cleared when the file is deleted, moved or becomes unreadable.
    pub available: bool,
}

impl ShareItem {
    fn from_path(absolute_path: PathBuf, display_name: String) -> Result<Self> {
        let metadata = std::fs::metadata(&absolute_path).map_err(|_| Error::FileUnavailable)?;
        Ok(ShareItem {
            id: tokens::file_id()?,
            display_name,
            mime_type: paths::guess_mime(&absolute_path),
            size: metadata.len(),
            added_at: now_millis(),
            available: true,
            absolute_path,
        })
    }
}

/// Aggregate figures for the "3 files · 185 MB shared" line.
#[derive(Debug, Clone, Copy, Serialize, Default, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RegistryTotals {
    pub file_count: usize,
    pub total_bytes: u64,
    pub unavailable_count: usize,
}

/// Outcome of an add operation, so the UI can explain partial success.
#[derive(Debug, Clone, Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct AddOutcome {
    pub added: Vec<ShareItem>,
    pub skipped_duplicates: usize,
    pub skipped_unreadable: usize,
    /// True when a dropped folder hit [`MAX_FILES_PER_FOLDER`].
    pub truncated: bool,
}

#[derive(Default)]
struct RegistryInner {
    /// Insertion order, so the list does not reshuffle under the user.
    order: Vec<String>,
    items: HashMap<String, ShareItem>,
    /// Canonical path -> id, for duplicate detection.
    by_path: HashMap<PathBuf, String>,
}

#[derive(Default)]
pub struct ShareRegistry {
    inner: RwLock<RegistryInner>,
}

impl ShareRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    /// Add files and/or folders that the user dropped or picked.
    ///
    /// Unreadable entries are counted and skipped rather than failing the
    /// whole batch: dropping ten files should not be undone by one bad one.
    pub fn add_paths<P: AsRef<Path>>(&self, inputs: &[P]) -> Result<AddOutcome> {
        let mut outcome = AddOutcome::default();
        let mut candidates: Vec<(PathBuf, String)> = Vec::new();

        for input in inputs {
            let input = input.as_ref();
            if input.is_dir() {
                match paths::canonicalize_shared_dir(input) {
                    Ok(root) => {
                        let (files, truncated) = walk_folder(&root);
                        outcome.truncated |= truncated;
                        for file in files {
                            let label = paths::relative_display_name(&root, &file);
                            candidates.push((file, label));
                        }
                    }
                    Err(_) => outcome.skipped_unreadable += 1,
                }
                continue;
            }

            match paths::canonicalize_shared_file(input) {
                Ok(canonical) => {
                    let label = canonical
                        .file_name()
                        .map(|n| paths::sanitize_filename(&n.to_string_lossy()))
                        .unwrap_or_else(|| "download".to_string());
                    candidates.push((canonical, label));
                }
                Err(_) => outcome.skipped_unreadable += 1,
            }
        }

        let mut guard = self.inner.write().map_err(poisoned)?;
        for (path, label) in candidates {
            if guard.by_path.contains_key(&path) {
                outcome.skipped_duplicates += 1;
                continue;
            }
            match ShareItem::from_path(path.clone(), label) {
                Ok(item) => {
                    guard.order.push(item.id.clone());
                    guard.by_path.insert(path, item.id.clone());
                    guard.items.insert(item.id.clone(), item.clone());
                    outcome.added.push(item);
                }
                Err(_) => outcome.skipped_unreadable += 1,
            }
        }
        Ok(outcome)
    }

    /// Remove one file. Its download URL stops working immediately, because
    /// every request resolves the id through this map.
    pub fn remove(&self, id: &str) -> Result<bool> {
        let mut guard = self.inner.write().map_err(poisoned)?;
        let Some(item) = guard.items.remove(id) else {
            return Ok(false);
        };
        guard.by_path.remove(&item.absolute_path);
        guard.order.retain(|existing| existing != id);
        Ok(true)
    }

    /// Empty the registry. This touches no file on disk.
    pub fn clear(&self) -> Result<usize> {
        let mut guard = self.inner.write().map_err(poisoned)?;
        let removed = guard.order.len();
        guard.order.clear();
        guard.items.clear();
        guard.by_path.clear();
        Ok(removed)
    }

    pub fn list(&self) -> Result<Vec<ShareItem>> {
        let guard = self.inner.read().map_err(poisoned)?;
        Ok(guard
            .order
            .iter()
            .filter_map(|id| guard.items.get(id).cloned())
            .collect())
    }

    /// Resolve a public id. Returns `None` for anything not registered, which
    /// is how every hostile or stale id is rejected.
    pub fn get(&self, id: &str) -> Result<Option<ShareItem>> {
        let guard = self.inner.read().map_err(poisoned)?;
        Ok(guard.items.get(id).cloned())
    }

    pub fn totals(&self) -> Result<RegistryTotals> {
        let guard = self.inner.read().map_err(poisoned)?;
        let mut totals = RegistryTotals::default();
        for item in guard.items.values() {
            totals.file_count += 1;
            if item.available {
                totals.total_bytes += item.size;
            } else {
                totals.unavailable_count += 1;
            }
        }
        Ok(totals)
    }

    /// Mark a single file unavailable after a failed open.
    pub fn mark_unavailable(&self, id: &str) -> Result<bool> {
        let mut guard = self.inner.write().map_err(poisoned)?;
        match guard.items.get_mut(id) {
            Some(item) if item.available => {
                item.available = false;
                Ok(true)
            }
            _ => Ok(false),
        }
    }

    /// Re-stat every entry. Returns true when anything changed, so the caller
    /// only pushes an event when there is news.
    pub fn refresh_availability(&self) -> Result<bool> {
        let mut guard = self.inner.write().map_err(poisoned)?;
        let mut changed = false;
        for item in guard.items.values_mut() {
            let (available, size) = match std::fs::metadata(&item.absolute_path) {
                Ok(meta) if meta.is_file() => (true, meta.len()),
                _ => (false, item.size),
            };
            if item.available != available || item.size != size {
                item.available = available;
                item.size = size;
                changed = true;
            }
        }
        Ok(changed)
    }

    /// Drop every entry whose file has gone away.
    pub fn prune_unavailable(&self) -> Result<usize> {
        let mut guard = self.inner.write().map_err(poisoned)?;
        let doomed: Vec<String> = guard
            .items
            .values()
            .filter(|item| !item.available)
            .map(|item| item.id.clone())
            .collect();
        for id in &doomed {
            if let Some(item) = guard.items.remove(id) {
                guard.by_path.remove(&item.absolute_path);
            }
        }
        guard.order.retain(|id| !doomed.contains(id));
        Ok(doomed.len())
    }

    pub fn is_empty(&self) -> bool {
        self.inner
            .read()
            .map(|guard| guard.order.is_empty())
            .unwrap_or(true)
    }
}

/// Iterative, depth-limited walk. Symlinked directories are not followed, so a
/// self-referential link cannot spin forever.
fn walk_folder(root: &Path) -> (Vec<PathBuf>, bool) {
    let mut files = Vec::new();
    let mut stack = vec![(root.to_path_buf(), 0usize)];
    let mut truncated = false;

    while let Some((dir, depth)) = stack.pop() {
        if depth > MAX_FOLDER_DEPTH {
            truncated = true;
            continue;
        }
        let Ok(entries) = std::fs::read_dir(&dir) else {
            continue;
        };
        for entry in entries.flatten() {
            if files.len() >= MAX_FILES_PER_FOLDER {
                truncated = true;
                return (files, truncated);
            }
            let path = entry.path();
            // `file_type` does not follow symlinks, which is what we want.
            let Ok(file_type) = entry.file_type() else {
                continue;
            };
            if file_type.is_symlink() {
                continue;
            }
            if file_type.is_dir() {
                stack.push((path, depth + 1));
            } else if file_type.is_file() && !is_hidden(&path) {
                files.push(path);
            }
        }
    }
    files.sort();
    (files, truncated)
}

fn is_hidden(path: &Path) -> bool {
    path.file_name()
        .and_then(|name| name.to_str())
        .map(|name| name.starts_with('.'))
        .unwrap_or(false)
}

pub(crate) fn now_millis() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

fn poisoned<T>(_: std::sync::PoisonError<T>) -> Error {
    Error::Internal("the shared-file registry lock was poisoned".into())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write(dir: &Path, name: &str, bytes: &[u8]) -> PathBuf {
        let path = dir.join(name);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).expect("mkdir");
        }
        std::fs::write(&path, bytes).expect("write");
        path
    }

    #[test]
    fn adding_files_assigns_opaque_ids_and_metadata() {
        let dir = tempfile::tempdir().expect("tempdir");
        let a = write(dir.path(), "report.pdf", b"pdf-bytes");
        let b = write(dir.path(), "clip.mp4", &vec![0u8; 4096]);

        let registry = ShareRegistry::new();
        let outcome = registry.add_paths(&[a, b]).expect("add");

        assert_eq!(outcome.added.len(), 2);
        assert_eq!(outcome.skipped_duplicates, 0);

        let items = registry.list().expect("list");
        assert_eq!(items.len(), 2);
        assert_eq!(items[0].display_name, "report.pdf");
        assert_eq!(items[0].mime_type, "application/pdf");
        assert_eq!(items[0].size, 9);
        assert_eq!(items[1].mime_type, "video/mp4");
        assert_eq!(items[1].size, 4096);

        // Ids must not leak the filename or the path.
        for item in &items {
            assert_eq!(item.id.len(), tokens::FILE_ID_LEN);
            assert!(!item.id.contains("report"));
            assert!(!item.id.contains('/'));
        }
        assert_ne!(items[0].id, items[1].id);
    }

    #[test]
    fn insertion_order_is_preserved() {
        let dir = tempfile::tempdir().expect("tempdir");
        let registry = ShareRegistry::new();
        for name in ["one.txt", "two.txt", "three.txt", "four.txt"] {
            let path = write(dir.path(), name, b"x");
            registry.add_paths(&[path]).expect("add");
        }
        let names: Vec<String> = registry
            .list()
            .expect("list")
            .into_iter()
            .map(|item| item.display_name)
            .collect();
        assert_eq!(names, ["one.txt", "two.txt", "three.txt", "four.txt"]);
    }

    #[test]
    fn the_same_file_is_not_added_twice() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = write(dir.path(), "a.txt", b"x");

        let registry = ShareRegistry::new();
        registry
            .add_paths(std::slice::from_ref(&path))
            .expect("add");
        let second = registry.add_paths(&[path]).expect("add again");

        assert_eq!(second.added.len(), 0);
        assert_eq!(second.skipped_duplicates, 1);
        assert_eq!(registry.list().expect("list").len(), 1);
    }

    #[test]
    fn unreadable_entries_are_skipped_without_failing_the_batch() {
        let dir = tempfile::tempdir().expect("tempdir");
        let good = write(dir.path(), "good.txt", b"x");
        let missing = dir.path().join("does-not-exist.txt");

        let registry = ShareRegistry::new();
        let outcome = registry.add_paths(&[good, missing]).expect("add");

        assert_eq!(outcome.added.len(), 1);
        assert_eq!(outcome.skipped_unreadable, 1);
    }

    #[test]
    fn removing_an_item_makes_its_id_unresolvable() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = write(dir.path(), "a.txt", b"x");
        let registry = ShareRegistry::new();
        let outcome = registry.add_paths(&[path]).expect("add");
        let id = outcome.added[0].id.clone();

        assert!(registry.get(&id).expect("get").is_some());
        assert!(registry.remove(&id).expect("remove"));
        assert!(registry.get(&id).expect("get").is_none());
        // Removing twice is not an error, it just reports nothing happened.
        assert!(!registry.remove(&id).expect("remove"));
    }

    #[test]
    fn unknown_and_hostile_ids_never_resolve() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = write(dir.path(), "a.txt", b"x");
        let registry = ShareRegistry::new();
        registry.add_paths(&[path]).expect("add");

        for hostile in [
            "",
            "..",
            "../../etc/passwd",
            "%2e%2e%2f",
            "/etc/passwd",
            "C:\\Windows\\System32\\config\\SAM",
            "0000000000000000",
            "' OR 1=1 --",
        ] {
            assert!(
                registry.get(hostile).expect("get").is_none(),
                "{hostile} resolved"
            );
        }
    }

    #[test]
    fn clear_empties_the_registry_but_leaves_files_on_disk() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = write(dir.path(), "a.txt", b"payload");
        let registry = ShareRegistry::new();
        registry
            .add_paths(std::slice::from_ref(&path))
            .expect("add");

        assert_eq!(registry.clear().expect("clear"), 1);
        assert!(registry.list().expect("list").is_empty());
        assert!(path.exists(), "clear must never delete the user's file");
        assert_eq!(std::fs::read(&path).expect("read"), b"payload");
    }

    #[test]
    fn a_removed_path_can_be_added_again() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = write(dir.path(), "a.txt", b"x");
        let registry = ShareRegistry::new();
        let first = registry
            .add_paths(std::slice::from_ref(&path))
            .expect("add");
        registry.remove(&first.added[0].id).expect("remove");

        let second = registry.add_paths(&[path]).expect("re-add");
        assert_eq!(second.added.len(), 1);
        assert_ne!(
            second.added[0].id, first.added[0].id,
            "a new id must be issued"
        );
    }

    #[test]
    fn deleted_files_are_detected_and_can_be_pruned() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = write(dir.path(), "gone.txt", b"x");
        let registry = ShareRegistry::new();
        registry
            .add_paths(std::slice::from_ref(&path))
            .expect("add");

        assert!(
            !registry.refresh_availability().expect("refresh"),
            "nothing changed yet"
        );
        std::fs::remove_file(&path).expect("remove file");

        assert!(registry.refresh_availability().expect("refresh"));
        assert!(!registry.list().expect("list")[0].available);
        assert_eq!(registry.prune_unavailable().expect("prune"), 1);
        assert!(registry.list().expect("list").is_empty());
    }

    #[test]
    fn totals_exclude_unavailable_bytes() {
        let dir = tempfile::tempdir().expect("tempdir");
        let a = write(dir.path(), "a.bin", &vec![0u8; 1000]);
        let b = write(dir.path(), "b.bin", &vec![0u8; 2000]);
        let registry = ShareRegistry::new();
        registry.add_paths(&[a, b.clone()]).expect("add");

        let totals = registry.totals().expect("totals");
        assert_eq!(totals.file_count, 2);
        assert_eq!(totals.total_bytes, 3000);
        assert_eq!(totals.unavailable_count, 0);

        std::fs::remove_file(&b).expect("rm");
        registry.refresh_availability().expect("refresh");
        let totals = registry.totals().expect("totals");
        assert_eq!(totals.file_count, 2);
        assert_eq!(totals.total_bytes, 1000);
        assert_eq!(totals.unavailable_count, 1);
    }

    #[test]
    fn dropping_a_folder_adds_its_files_with_relative_labels() {
        let dir = tempfile::tempdir().expect("tempdir");
        let root = dir.path().join("photos");
        write(&root, "a.jpg", b"1");
        write(&root.join("2026"), "b.jpg", b"2");
        write(&root, ".hidden", b"3");

        let registry = ShareRegistry::new();
        let outcome = registry.add_paths(&[root]).expect("add folder");

        let mut names: Vec<String> = outcome
            .added
            .iter()
            .map(|item| item.display_name.clone())
            .collect();
        names.sort();
        assert_eq!(names, ["photos/2026/b.jpg", "photos/a.jpg"]);
        assert!(
            !names.iter().any(|n| n.contains("hidden")),
            "dotfiles are skipped"
        );
    }

    #[test]
    fn a_directory_symlink_loop_terminates() {
        let dir = tempfile::tempdir().expect("tempdir");
        let root = dir.path().join("loop");
        write(&root, "real.txt", b"x");
        #[cfg(unix)]
        std::os::unix::fs::symlink(&root, root.join("self")).expect("symlink");

        let registry = ShareRegistry::new();
        let outcome = registry.add_paths(&[root]).expect("add folder");
        assert_eq!(outcome.added.len(), 1);
    }

    #[test]
    fn the_registry_is_usable_from_many_threads() {
        let dir = tempfile::tempdir().expect("tempdir");
        let registry = std::sync::Arc::new(ShareRegistry::new());
        let mut handles = Vec::new();

        for index in 0..8 {
            let path = write(dir.path(), &format!("f{index}.bin"), b"x");
            let registry = registry.clone();
            handles.push(std::thread::spawn(move || {
                registry.add_paths(&[path]).expect("add");
            }));
        }
        for handle in handles {
            handle.join().expect("join");
        }
        assert_eq!(registry.list().expect("list").len(), 8);
    }
}
