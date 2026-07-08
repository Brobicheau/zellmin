use std::collections::BTreeSet;
use std::io;

use host_storage::HostStorage;

const TREEMIN_NAMESPACE: &str = "treemin";
const TREEMIN_SESSIONS_FILE: &str = "sessions/managed.txt";

#[derive(Debug, Clone)]
pub struct TreeminSessionRegistry {
    storage: HostStorage,
}

impl TreeminSessionRegistry {
    pub fn new() -> io::Result<Self> {
        Ok(Self {
            storage: HostStorage::new(TREEMIN_NAMESPACE)?,
        })
    }

    pub fn with_storage(storage: HostStorage) -> Self {
        Self { storage }
    }

    pub fn list(&self) -> io::Result<BTreeSet<String>> {
        match self.storage.read_string(TREEMIN_SESSIONS_FILE) {
            Ok(contents) => Ok(parse_session_names(&contents)),
            Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(BTreeSet::new()),
            Err(error) => Err(error),
        }
    }

    pub fn contains(&self, session_name: &str) -> io::Result<bool> {
        Ok(self.list()?.contains(session_name))
    }

    pub fn add(&self, session_name: &str) -> io::Result<()> {
        let mut session_names = self.list()?;
        session_names.insert(session_name.to_string());
        self.write_all(&session_names)
    }

    pub fn remove(&self, session_name: &str) -> io::Result<()> {
        let mut session_names = self.list()?;
        session_names.remove(session_name);
        self.write_all(&session_names)
    }

    fn write_all(&self, session_names: &BTreeSet<String>) -> io::Result<()> {
        self.storage.ensure_root()?;
        let contents = if session_names.is_empty() {
            String::new()
        } else {
            let mut lines = session_names.iter().cloned().collect::<Vec<_>>();
            lines.push(String::new());
            lines.join("\n")
        };
        self.storage
            .write_string(TREEMIN_SESSIONS_FILE, &contents)?;
        Ok(())
    }
}

fn parse_session_names(contents: &str) -> BTreeSet<String> {
    contents
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(|line| line.to_string())
        .collect()
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    use super::*;

    fn registry() -> TreeminSessionRegistry {
        let unique_root = std::env::temp_dir().join(format!(
            "treemin-session-registry-{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let _ = fs::remove_dir_all(&unique_root);
        TreeminSessionRegistry::with_storage(
            HostStorage::with_host_root(PathBuf::from(unique_root), "treemin-test").unwrap(),
        )
    }

    #[test]
    fn adds_and_lists_managed_sessions() {
        let registry = registry();

        registry.add("repo-feature-a").unwrap();
        registry.add("repo-feature-b").unwrap();

        assert_eq!(
            registry.list().unwrap(),
            BTreeSet::from(["repo-feature-a".to_string(), "repo-feature-b".to_string(),])
        );
    }

    #[test]
    fn remove_deletes_managed_session_entry() {
        let registry = registry();

        registry.add("repo-feature-a").unwrap();
        registry.remove("repo-feature-a").unwrap();

        assert!(!registry.contains("repo-feature-a").unwrap());
    }

    #[test]
    fn missing_file_is_treated_as_empty_registry() {
        let registry = registry();

        assert!(registry.list().unwrap().is_empty());
    }
}
