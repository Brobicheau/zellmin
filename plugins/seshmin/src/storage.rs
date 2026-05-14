use session_registry::TreeminSessionRegistry;

pub fn treemin_registry() -> Option<TreeminSessionRegistry> {
    TreeminSessionRegistry::new().ok()
}

#[cfg(test)]
pub fn test_treemin_registry(root: impl AsRef<std::path::Path>) -> TreeminSessionRegistry {
    TreeminSessionRegistry::with_storage(
        host_storage::HostStorage::with_host_root(root, "treemin-test").unwrap(),
    )
}
