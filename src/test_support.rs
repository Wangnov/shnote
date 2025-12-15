use std::ffi::{OsStr, OsString};
use std::path::Path;
use std::sync::{Mutex, MutexGuard};

/// Global lock for tests that mutate process-wide environment variables.
static ENV_LOCK: Mutex<()> = Mutex::new(());

pub fn env_lock() -> MutexGuard<'static, ()> {
    ENV_LOCK.lock().expect("env lock should not be poisoned")
}

pub struct EnvVarGuard {
    key: String,
    prev: Option<OsString>,
}

impl EnvVarGuard {
    pub fn set(key: &str, value: impl AsRef<OsStr>) -> Self {
        let prev = std::env::var_os(key);
        std::env::set_var(key, value);
        Self {
            key: key.to_string(),
            prev,
        }
    }

    pub fn remove(key: &str) -> Self {
        let prev = std::env::var_os(key);
        std::env::remove_var(key);
        Self {
            key: key.to_string(),
            prev,
        }
    }
}

impl Drop for EnvVarGuard {
    fn drop(&mut self) {
        match &self.prev {
            Some(v) => std::env::set_var(&self.key, v),
            None => std::env::remove_var(&self.key),
        }
    }
}

#[cfg(unix)]
pub fn write_executable(path: &Path, contents: &str) -> std::io::Result<()> {
    use std::fs;
    use std::os::unix::fs::PermissionsExt;

    fs::write(path, contents)?;
    let perms = fs::Permissions::from_mode(0o755);
    fs::set_permissions(path, perms)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn env_var_guard_restores_previous_value() {
        let _lock = env_lock();

        std::env::remove_var("SHNOTE_TEST_ENV");
        {
            let _g = EnvVarGuard::set("SHNOTE_TEST_ENV", "value1");
            assert_eq!(std::env::var("SHNOTE_TEST_ENV").unwrap(), "value1");
        }
        assert!(std::env::var("SHNOTE_TEST_ENV").is_err());

        std::env::set_var("SHNOTE_TEST_ENV", "prev");
        {
            let _g = EnvVarGuard::remove("SHNOTE_TEST_ENV");
            assert!(std::env::var("SHNOTE_TEST_ENV").is_err());
        }
        assert_eq!(std::env::var("SHNOTE_TEST_ENV").unwrap(), "prev");
    }

    #[cfg(unix)]
    #[test]
    fn write_executable_errors_when_path_is_directory() {
        let dir = tempfile::TempDir::new().unwrap();
        let err = write_executable(dir.path(), "#!/bin/sh\nexit 0\n").unwrap_err();
        assert_eq!(err.kind(), std::io::ErrorKind::IsADirectory);
    }

    #[cfg(unix)]
    #[test]
    fn write_executable_errors_when_cannot_set_permissions() {
        assert!(write_executable(Path::new("/dev/null"), "x").is_err());
    }
}
