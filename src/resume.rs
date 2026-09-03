use std::{
    fs, io,
    path::{Path, PathBuf},
    time::{Duration, SystemTime, UNIX_EPOCH},
};

#[derive(Debug, Clone)]
pub struct ResumeCache {
    directory: PathBuf,
    ttl: Duration,
    enabled: bool,
}

impl ResumeCache {
    #[must_use]
    pub fn new(enabled: bool, ttl_minutes: u64) -> Self {
        let directory = dirs::cache_dir()
            .unwrap_or_else(std::env::temp_dir)
            .join("mdt")
            .join("positions");
        Self {
            directory,
            ttl: Duration::from_secs(ttl_minutes.saturating_mul(60)),
            enabled,
        }
    }

    pub fn save(&self, path: &str, source_line: usize) -> io::Result<()> {
        if !self.enabled || self.ttl.is_zero() {
            return Ok(());
        }

        fs::create_dir_all(&self.directory)?;
        let cache_file = self.cache_file(path);
        let temporary_file = cache_file.with_extension(format!("tmp-{}", std::process::id()));
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        fs::write(&temporary_file, format!("{timestamp}\n{source_line}\n"))?;
        fs::rename(temporary_file, cache_file)
    }

    #[must_use]
    pub fn load(&self, path: &str) -> Option<usize> {
        if !self.enabled || self.ttl.is_zero() {
            return None;
        }

        let contents = fs::read_to_string(self.cache_file(path)).ok()?;
        parse_record(&contents, SystemTime::now(), self.ttl)
    }

    fn cache_file(&self, path: &str) -> PathBuf {
        let canonical = Path::new(path)
            .canonicalize()
            .unwrap_or_else(|_| PathBuf::from(path));
        let hash = canonical
            .to_string_lossy()
            .bytes()
            .fold(0xcbf2_9ce4_8422_2325_u64, |hash, byte| {
                (hash ^ u64::from(byte)).wrapping_mul(0x1000_0000_01b3)
            });
        self.directory.join(format!("{hash:016x}"))
    }
}

fn parse_record(contents: &str, now: SystemTime, ttl: Duration) -> Option<usize> {
    let mut lines = contents.lines();
    let timestamp = lines.next()?.parse::<u64>().ok()?;
    let source_line = lines.next()?.parse::<usize>().ok()?;
    let saved_at = UNIX_EPOCH.checked_add(Duration::from_secs(timestamp))?;
    if now.duration_since(saved_at).ok()? > ttl {
        return None;
    }
    Some(source_line)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_a_fresh_position() {
        let now = UNIX_EPOCH + Duration::from_secs(1_060);
        assert_eq!(
            parse_record("1000\n123\n", now, Duration::from_secs(60)),
            Some(123)
        );
    }

    #[test]
    fn ignores_an_expired_position() {
        let now = UNIX_EPOCH + Duration::from_secs(1_061);
        assert_eq!(
            parse_record("1000\n123\n", now, Duration::from_secs(60)),
            None
        );
    }

    #[test]
    fn disabled_cache_does_not_read() {
        let cache = ResumeCache {
            directory: PathBuf::from("unused"),
            ttl: Duration::from_secs(60),
            enabled: false,
        };
        assert_eq!(cache.load("document.md"), None);
    }
}
