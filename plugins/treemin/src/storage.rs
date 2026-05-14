use session_registry::TreeminSessionRegistry;

pub fn treemin_registry() -> Option<TreeminSessionRegistry> {
    TreeminSessionRegistry::new().ok()
}
