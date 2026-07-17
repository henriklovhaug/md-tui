use std::{
    collections::BTreeMap,
    fs,
    io::{self, Write},
    path::{Path, PathBuf},
};

use serde::{Deserialize, Serialize};

use crate::util::Caret;

#[derive(Debug, Default, Serialize, Deserialize)]
struct Store {
    #[serde(default)]
    files: BTreeMap<String, FileEntry>,
}

#[derive(Debug, Default, Serialize, Deserialize)]
struct FileEntry {
    // `default` so an older or hand-edited entry missing `width` doesn't fail
    // the whole store's deserialization (which would silently drop every
    // bookmark for every file on the next save).
    #[serde(default)]
    width: u16,
    #[serde(default)]
    marks: BTreeMap<String, [u16; 2]>,
}

pub fn store_path() -> Option<PathBuf> {
    if let Ok(p) = std::env::var("MDT_BOOKMARKS_FILE") {
        return Some(PathBuf::from(p));
    }
    let base = dirs::state_dir().or_else(dirs::data_local_dir)?;
    Some(base.join("mdt").join("bookmarks.toml"))
}

pub fn load_for(file: &Path) -> (BTreeMap<char, Caret>, u16) {
    let key = match canonical_key(file) {
        Some(k) => k,
        None => return (BTreeMap::new(), 0),
    };
    let path = match store_path() {
        Some(p) => p,
        None => return (BTreeMap::new(), 0),
    };
    let raw = match fs::read_to_string(&path) {
        Ok(s) => s,
        Err(_) => return (BTreeMap::new(), 0),
    };
    let store: Store = toml::from_str(&raw).unwrap_or_else(|e| {
        eprintln!(
            "warning: failed to parse bookmarks file {}: {e}",
            path.display()
        );
        Store::default()
    });
    let entry = match store.files.get(&key) {
        Some(e) => e,
        None => return (BTreeMap::new(), 0),
    };
    let mut marks = BTreeMap::new();
    for (k, [line, col]) in &entry.marks {
        if let Some(c) = single_lowercase_char(k) {
            marks.insert(
                c,
                Caret {
                    line: *line,
                    col: *col,
                },
            );
        }
    }
    (marks, entry.width)
}

pub fn save_for(file: &Path, marks: &BTreeMap<char, Caret>, width: u16) -> io::Result<()> {
    let key = canonical_key(file)
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "non-utf8 file path"))?;
    let path = store_path()
        .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "no state or data-local dir"))?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }

    let mut store: Store = match fs::read_to_string(&path) {
        Ok(s) => toml::from_str(&s).unwrap_or_else(|e| {
            eprintln!(
                "warning: failed to parse bookmarks file {}: {e}",
                path.display()
            );
            Store::default()
        }),
        Err(e) if e.kind() == io::ErrorKind::NotFound => Store::default(),
        Err(e) => return Err(e),
    };

    if marks.is_empty() {
        store.files.remove(&key);
    } else {
        let mut entry = FileEntry {
            width,
            marks: BTreeMap::new(),
        };
        for (c, caret) in marks {
            entry.marks.insert(c.to_string(), [caret.line, caret.col]);
        }
        store.files.insert(key, entry);
    }

    let serialized = toml::to_string(&store).map_err(io::Error::other)?;

    let tmp = path.with_extension("toml.tmp");
    {
        let mut f = fs::File::create(&tmp)?;
        f.write_all(serialized.as_bytes())?;
        f.sync_all()?;
    }
    fs::rename(&tmp, &path)?;
    Ok(())
}

fn canonical_key(file: &Path) -> Option<String> {
    // Canonicalize so bookmarks survive `cd` and the like. Falling back to
    // the raw path means a broken symlink (or one we can't resolve) gets
    // stored under a path-spelling-specific key — warn the user once so
    // they can spot when bookmarks split across spellings of the same file.
    let abs = match fs::canonicalize(file) {
        Ok(p) => p,
        Err(e) => {
            eprintln!(
                "warning: could not canonicalize {} for bookmark key: {e}",
                file.display()
            );
            file.to_path_buf()
        }
    };
    abs.to_str().map(|s| s.to_owned())
}

fn single_lowercase_char(s: &str) -> Option<char> {
    let mut chars = s.chars();
    let c = chars.next()?;
    if chars.next().is_some() || !c.is_ascii_lowercase() {
        return None;
    }
    Some(c)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{env, sync::Mutex};

    static TEST_ENV: Mutex<()> = Mutex::new(());

    fn unique_tmp(name: &str) -> PathBuf {
        let mut p = env::temp_dir();
        p.push(format!(
            "mdt-bookmarks-test-{}-{}-{}.tmp",
            name,
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        p
    }

    #[test]
    fn round_trip_marks() {
        let _guard = TEST_ENV.lock().unwrap();
        let store = unique_tmp("store");
        // SAFETY: env access serialized by TEST_ENV mutex above.
        unsafe { env::set_var("MDT_BOOKMARKS_FILE", &store) };
        let _ = fs::remove_file(&store);

        let file = unique_tmp("file");
        fs::write(&file, b"hello").unwrap();

        let mut marks = BTreeMap::new();
        marks.insert('a', Caret { line: 12, col: 34 });
        marks.insert('z', Caret { line: 0, col: 0 });

        save_for(&file, &marks, 100).unwrap();
        let (loaded, width) = load_for(&file);
        assert_eq!(width, 100);
        assert_eq!(loaded.get(&'a'), Some(&Caret { line: 12, col: 34 }));
        assert_eq!(loaded.get(&'z'), Some(&Caret { line: 0, col: 0 }));

        let empty = BTreeMap::new();
        save_for(&file, &empty, 100).unwrap();
        let (loaded, width) = load_for(&file);
        assert!(loaded.is_empty());
        assert_eq!(width, 0);

        let _ = fs::remove_file(&store);
        let _ = fs::remove_file(&file);
        // SAFETY: env access serialized by TEST_ENV mutex above.
        unsafe { env::remove_var("MDT_BOOKMARKS_FILE") };
    }

    #[test]
    fn drops_invalid_keys() {
        assert!(single_lowercase_char("ab").is_none());
        assert!(single_lowercase_char("A").is_none());
        assert!(single_lowercase_char("1").is_none());
        assert_eq!(single_lowercase_char("a"), Some('a'));
    }
}
