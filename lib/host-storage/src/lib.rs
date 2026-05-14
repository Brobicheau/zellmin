use std::fs;
use std::io;
use std::path::{Component, Path, PathBuf};

pub const HOST_ROOT: &str = "/tmp";

#[derive(Debug, Clone)]
pub struct HostStorage {
    root: PathBuf,
}

impl HostStorage {
    pub fn new(namespace: &str) -> io::Result<Self> {
        Self::with_host_root(HOST_ROOT, namespace)
    }

    pub fn with_host_root(host_root: impl AsRef<Path>, namespace: &str) -> io::Result<Self> {
        let namespace = validate_relative_path(namespace)?;
        Ok(Self {
            root: host_root.as_ref().join(namespace),
        })
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn ensure_root(&self) -> io::Result<()> {
        fs::create_dir_all(&self.root)
    }

    pub fn exists(&self, relative_path: &str) -> io::Result<bool> {
        Ok(self.path(relative_path)?.exists())
    }

    pub fn path(&self, relative_path: &str) -> io::Result<PathBuf> {
        let relative_path = validate_relative_path(relative_path)?;
        Ok(self.root.join(relative_path))
    }

    pub fn create_dir_all(&self, relative_path: &str) -> io::Result<PathBuf> {
        let path = self.path(relative_path)?;
        fs::create_dir_all(&path)?;
        Ok(path)
    }

    pub fn read_string(&self, relative_path: &str) -> io::Result<String> {
        fs::read_to_string(self.path(relative_path)?)
    }

    pub fn write_string(&self, relative_path: &str, contents: &str) -> io::Result<PathBuf> {
        let path = self.path(relative_path)?;
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(&path, contents)?;
        Ok(path)
    }

    pub fn remove_file(&self, relative_path: &str) -> io::Result<()> {
        match fs::remove_file(self.path(relative_path)?) {
            Ok(()) => Ok(()),
            Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
            Err(error) => Err(error),
        }
    }
}

fn validate_relative_path(path: &str) -> io::Result<&Path> {
    let path = Path::new(path);
    if path.as_os_str().is_empty() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "path must not be empty",
        ));
    }

    if path.is_absolute() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "path must be relative",
        ));
    }

    if path.components().any(|component| {
        matches!(
            component,
            Component::ParentDir | Component::RootDir | Component::Prefix(_)
        )
    }) {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "path must stay within the host storage root",
        ));
    }

    Ok(path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builds_namespace_under_host_root() {
        let storage = HostStorage::new("treemin").unwrap();

        assert_eq!(storage.root(), Path::new("/tmp/treemin"));
    }

    #[test]
    fn rejects_absolute_namespace() {
        let error = HostStorage::new("/tmp/nope").unwrap_err();

        assert_eq!(error.kind(), io::ErrorKind::InvalidInput);
    }

    #[test]
    fn rejects_parent_directory_escape() {
        let error = HostStorage::new("../escape").unwrap_err();

        assert_eq!(error.kind(), io::ErrorKind::InvalidInput);
    }

    #[test]
    fn joins_relative_paths_under_namespace() {
        let storage = HostStorage::with_host_root("/tmp/host", "seshmin").unwrap();

        assert_eq!(
            storage.path("state/cache.json").unwrap(),
            PathBuf::from("/tmp/host/seshmin/state/cache.json")
        );
    }

    #[test]
    fn rejects_relative_paths_that_escape_namespace() {
        let storage = HostStorage::with_host_root("/tmp/host", "seshmin").unwrap();
        let error = storage.path("../cache.json").unwrap_err();

        assert_eq!(error.kind(), io::ErrorKind::InvalidInput);
    }
}
