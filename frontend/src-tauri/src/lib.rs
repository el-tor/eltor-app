mod app_paths;

#[cfg(test)]
mod tests {
    use crate::app_paths::initialize_torrc_files;
    
    #[test]
    fn test_torrc_initialization() {
        // This test will run from the src-tauri directory
        match initialize_torrc_files() {
            Ok(()) => println!("✅ torrc files initialized successfully"),
            Err(e) => panic!("❌ Failed to initialize torrc files: {}", e),
        }
    }
}
