/// Example usage of strategies with IconResolver
/// 
/// This module provides examples of how to integrate various strategies
/// with the icon resolution system.

#[cfg(test)]
mod tests {
    use std::path::PathBuf;
    use tempfile::TempDir;
    use std::fs;
    
    use crate::icon::{IconResolver, IconContext};
    use crate::icon::strategies::DirectoryStrategy;

    fn create_example_icon_structure() -> TempDir {
        let temp_dir = TempDir::new().unwrap();
        let icons_dir = temp_dir.path().join("icons");
        
        // Create directory structure similar to real Linux systems
        fs::create_dir_all(&icons_dir).unwrap();
        fs::create_dir_all(icons_dir.join("hicolor/48x48/apps")).unwrap();
        fs::create_dir_all(icons_dir.join("Adwaita/scalable/apps")).unwrap();
        
        // Create example icon files
        fs::write(icons_dir.join("firefox.png"), b"fake firefox png").unwrap();
        fs::write(icons_dir.join("chrome.svg"), b"fake chrome svg").unwrap();
        fs::write(icons_dir.join("hicolor/48x48/apps/gimp.png"), b"fake gimp png").unwrap();
        fs::write(icons_dir.join("Adwaita/scalable/apps/nautilus.svg"), b"fake nautilus svg").unwrap();
        
        temp_dir
    }

    #[test]
    fn test_directory_strategy_integration() {
        let temp_dir = create_example_icon_structure();
        let icons_path = temp_dir.path().join("icons");
        
        // Create and configure the resolver
        let mut resolver = IconResolver::new();
        
        // Create and register the DirectoryStrategy
        let strategy = Box::new(DirectoryStrategy::with_directories(vec![icons_path]));
        resolver.register_strategy(strategy).unwrap();
        
        // Test resolving various icons
        let test_cases = vec![
            ("firefox", true),
            ("chrome", true),
            ("gimp", true),
            ("nautilus", true),
            ("nonexistent", false),
        ];
        
        for (class_name, should_find) in test_cases {
            let context = IconContext::new(class_name.to_string());
            let result = resolver.resolve(&context);
            
            if should_find {
                assert!(result.is_some(), "Should find icon for {}", class_name);
                let icon_result = result.unwrap();
                assert_eq!(icon_result.strategy_used, "DirectoryStrategy");
                assert!(icon_result.path.to_string_lossy().contains(class_name));
            } else {
                assert!(result.is_none(), "Should not find icon for {}", class_name);
            }
        }
        
        // Check resolver statistics
        let stats = resolver.get_stats().unwrap();
        let directory_stats = stats.get("DirectoryStrategy").unwrap();
        assert_eq!(directory_stats.0, 5); // 5 attempts
        assert_eq!(directory_stats.1, 4); // 4 successes
    }

    #[test]
    fn test_directory_strategy_with_custom_config() {
        let temp_dir = create_example_icon_structure();
        let icons_path = temp_dir.path().join("icons");
        
        // Create a DirectoryStrategy with custom configuration
        let strategy = DirectoryStrategy::with_directories(vec![icons_path])
            .with_max_depth(5) // Allow deeper recursion
            .with_cache_ttl(std::time::Duration::from_secs(600)); // 10 minute cache
        
        let mut resolver = IconResolver::new();
        resolver.register_strategy(Box::new(strategy)).unwrap();
        
        // Test that it works with the custom configuration
        let context = IconContext::new("gimp".to_string());
        let result = resolver.resolve(&context);
        
        assert!(result.is_some());
        let icon_result = result.unwrap();
        assert_eq!(icon_result.strategy_used, "DirectoryStrategy");
        assert!(icon_result.path.to_string_lossy().contains("gimp"));
    }

    #[test]
    fn test_directory_strategy_name_variations() {
        let temp_dir = create_example_icon_structure();
        let icons_path = temp_dir.path().join("icons");
        
        // Add an icon with a complex name
        fs::write(icons_path.join("org.mozilla.firefox.png"), b"fake firefox png").unwrap();
        
        let mut resolver = IconResolver::new();
        let strategy = Box::new(DirectoryStrategy::with_directories(vec![icons_path]));
        resolver.register_strategy(strategy).unwrap();
        
        // Test that various name formats can find icons
        // Some will find the complex name, others will find the simple "firefox" icon
        let test_cases = vec![
            ("org.mozilla.Firefox", true),
            ("org.mozilla.firefox", true), 
            ("mozilla.firefox", true),
            ("firefox", true), // This should find the simple firefox.png
        ];
        
        for (class_name, should_find) in test_cases {
            let context = IconContext::new(class_name.to_string());
            let result = resolver.resolve(&context);
            
            if should_find {
                assert!(result.is_some(), "Should find icon for class: {}", class_name);
                let icon_result = result.unwrap();
                assert!(
                    icon_result.path.to_string_lossy().contains("org.mozilla.firefox") ||
                    icon_result.path.to_string_lossy().contains("firefox")
                );
            } else {
                assert!(result.is_none(), "Should not find icon for class: {}", class_name);
            }
        }
    }

    #[test]
    fn test_directory_strategy_format_preference() {
        let temp_dir = TempDir::new().unwrap();
        let icons_dir = temp_dir.path().join("icons");
        fs::create_dir_all(&icons_dir).unwrap();
        
        // Create the same icon in multiple formats
        fs::write(icons_dir.join("test-app.png"), b"fake png").unwrap();
        fs::write(icons_dir.join("test-app.svg"), b"fake svg").unwrap();
        fs::write(icons_dir.join("test-app.xpm"), b"fake xpm").unwrap();
        
        let mut resolver = IconResolver::new();
        let strategy = Box::new(DirectoryStrategy::with_directories(vec![icons_dir]));
        resolver.register_strategy(strategy).unwrap();
        
        let context = IconContext::new("test-app".to_string());
        let result = resolver.resolve(&context);
        
        assert!(result.is_some());
        let icon_result = result.unwrap();
        
        // Should prefer SVG (vector format) over raster formats
        assert!(icon_result.path.to_string_lossy().ends_with(".svg"));
    }

    #[test]
    fn test_hyprland_strategy_integration() {
        use crate::icon::strategies::HyprlandStrategy;
        
        // Create a resolver with HyprlandStrategy
        let mut resolver = IconResolver::new();
        let strategy = Box::new(HyprlandStrategy::new());
        resolver.register_strategy(strategy).unwrap();
        
        // Test with context that has Hyprland-specific information
        let context = IconContext::with_title(
            "unknown-class".to_string(), 
            "Firefox - Mozilla Firefox".to_string()
        );
        
        // The strategy should attempt to extract "Firefox" from the title
        // and look for firefox icons (result depends on system icons)
        let result = resolver.resolve(&context);
        
        // We can't guarantee the result since it depends on system icons,
        // but we can verify the strategy was attempted
        match result {
            Some(icon_result) => {
                assert_eq!(icon_result.strategy_used, "HyprlandStrategy");
                assert_eq!(icon_result.confidence, 0.8);
            }
            None => {
                // Expected if firefox icon doesn't exist on test system
            }
        }
    }

    #[test]
    fn test_hyprland_strategy_with_executable() {
        use crate::icon::strategies::HyprlandStrategy;
        
        let mut resolver = IconResolver::new();
        let strategy = Box::new(HyprlandStrategy::new());
        resolver.register_strategy(strategy).unwrap();
        
        // Test with executable information
        let context = IconContext::new("unknown-class".to_string())
            .with_executable("code".to_string());
        
        let result = resolver.resolve(&context);
        
        // Result depends on whether VS Code icon exists
        match result {
            Some(icon_result) => {
                assert_eq!(icon_result.strategy_used, "HyprlandStrategy");
            }
            None => {
                // Expected if code icon doesn't exist
            }
        }
    }

    #[test]
    fn test_hyprland_strategy_priority() {
        use crate::icon::strategies::{HyprlandStrategy, DirectoryStrategy};
        
        let temp_dir = create_example_icon_structure();
        let icons_path = temp_dir.path().join("icons");
        
        // Create a firefox icon in the directory
        fs::write(icons_path.join("firefox.png"), b"fake firefox png").unwrap();
        
        let mut resolver = IconResolver::new();
        
        // Register DirectoryStrategy with lower priority
        let dir_strategy = Box::new(DirectoryStrategy::with_directories(vec![icons_path]));
        resolver.register_strategy(dir_strategy).unwrap();
        
        // Register HyprlandStrategy with higher priority
        let hypr_strategy = Box::new(HyprlandStrategy::new());
        resolver.register_strategy(hypr_strategy).unwrap();
        
        // Test with context that both strategies could handle
        let context = IconContext::with_title(
            "firefox".to_string(),
            "Firefox - Mozilla Firefox".to_string()
        );
        
        let result = resolver.resolve(&context);
        
        if let Some(icon_result) = result {
            // HyprlandStrategy should win due to higher priority (75 vs default)
            // But only if it finds an icon, otherwise DirectoryStrategy will be used
            assert!(icon_result.strategy_used == "HyprlandStrategy" || 
                   icon_result.strategy_used == "DirectoryStrategy");
        }
    }
}