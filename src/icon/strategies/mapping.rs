use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use tracing::{debug, warn};

use crate::icon::types::{IconContext, IconResult, IconMetadata, IconFormat};
use crate::icon::traits::IconDetectionStrategy;

/// Component that manages application class to icon name mappings
/// 
/// This component handles the mapping of window class names to appropriate icon names.
/// It supports multiple aliases per application and can be configured with custom mappings.
#[derive(Debug, Clone)]
pub struct ApplicationMapper {
    /// Map from window class to possible icon names (in order of preference)
    class_mappings: HashMap<String, Vec<String>>,
    /// Reverse mapping from icon name to preferred class (for optimization)
    reverse_mappings: HashMap<String, String>,
}

impl ApplicationMapper {
    /// Create a new ApplicationMapper with default mappings
    pub fn new() -> Self {
        let mut mapper = Self {
            class_mappings: HashMap::new(),
            reverse_mappings: HashMap::new(),
        };
        
        mapper.load_default_mappings();
        mapper
    }

    /// Create an ApplicationMapper with custom mappings only
    pub fn with_custom_mappings(mappings: HashMap<String, Vec<String>>) -> Self {
        let mut mapper = Self {
            class_mappings: HashMap::new(),
            reverse_mappings: HashMap::new(),
        };
        
        for (class, aliases) in mappings {
            mapper.add_mapping(class, aliases);
        }
        
        mapper
    }

    /// Add a mapping from window class to icon names
    /// 
    /// # Arguments
    /// * `class` - The window class name
    /// * `icon_names` - List of possible icon names in order of preference
    pub fn add_mapping(&mut self, class: String, icon_names: Vec<String>) {
        if icon_names.is_empty() {
            warn!("ApplicationMapper: Empty icon names list for class '{}'", class);
            return;
        }

        // Add reverse mapping for the first (preferred) icon name
        if let Some(first_icon) = icon_names.first() {
            self.reverse_mappings.insert(first_icon.clone(), class.clone());
        }

        self.class_mappings.insert(class, icon_names);
    }

    /// Add a single alias for a window class
    pub fn add_alias(&mut self, class: String, icon_name: String) {
        self.class_mappings
            .entry(class)
            .or_insert_with(Vec::new)
            .push(icon_name);
    }

    /// Get possible icon names for a window class
    pub fn get_icon_names(&self, class: &str) -> Option<&Vec<String>> {
        // Try exact match first
        if let Some(names) = self.class_mappings.get(class) {
            return Some(names);
        }

        // Try case-insensitive match
        let class_lower = class.to_lowercase();
        for (mapped_class, names) in &self.class_mappings {
            if mapped_class.to_lowercase() == class_lower {
                return Some(names);
            }
        }

        None
    }

    /// Get the preferred class name for an icon name (reverse lookup)
    pub fn get_preferred_class(&self, icon_name: &str) -> Option<&String> {
        self.reverse_mappings.get(icon_name)
    }

    /// Check if a class has any mappings
    pub fn has_mapping(&self, class: &str) -> bool {
        self.get_icon_names(class).is_some()
    }

    /// Get all mapped classes
    pub fn get_all_classes(&self) -> Vec<&String> {
        self.class_mappings.keys().collect()
    }

    /// Get the number of mappings
    pub fn mapping_count(&self) -> usize {
        self.class_mappings.len()
    }

    /// Load default mappings for common applications
    fn load_default_mappings(&mut self) {
        // Web browsers
        self.add_mapping("firefox".to_string(), vec![
            "firefox".to_string(),
            "Firefox".to_string(),
            "mozilla-firefox".to_string(),
        ]);
        
        self.add_mapping("Google-chrome".to_string(), vec![
            "google-chrome".to_string(),
            "chrome".to_string(),
            "chromium".to_string(),
        ]);
        
        self.add_mapping("chromium-browser".to_string(), vec![
            "chromium".to_string(),
            "chromium-browser".to_string(),
            "chrome".to_string(),
        ]);
        
        self.add_mapping("brave-browser".to_string(), vec![
            "brave".to_string(),
            "brave-browser".to_string(),
        ]);

        // Text editors and IDEs
        self.add_mapping("code".to_string(), vec![
            "vscode".to_string(),
            "code".to_string(),
            "visual-studio-code".to_string(),
        ]);
        
        self.add_mapping("Code".to_string(), vec![
            "vscode".to_string(),
            "code".to_string(),
            "visual-studio-code".to_string(),
        ]);
        
        self.add_mapping("nvim".to_string(), vec![
            "nvim".to_string(),
            "neovim".to_string(),
            "vim".to_string(),
        ]);
        
        self.add_mapping("emacs".to_string(), vec![
            "emacs".to_string(),
            "gnu-emacs".to_string(),
        ]);

        // Terminals
        self.add_mapping("Alacritty".to_string(), vec![
            "alacritty".to_string(),
            "terminal".to_string(),
        ]);
        
        self.add_mapping("kitty".to_string(), vec![
            "kitty".to_string(),
            "terminal".to_string(),
        ]);
        
        self.add_mapping("gnome-terminal".to_string(), vec![
            "gnome-terminal".to_string(),
            "terminal".to_string(),
        ]);
        
        self.add_mapping("konsole".to_string(), vec![
            "konsole".to_string(),
            "terminal".to_string(),
        ]);

        // File managers
        self.add_mapping("nautilus".to_string(), vec![
            "nautilus".to_string(),
            "file-manager".to_string(),
            "folder".to_string(),
        ]);
        
        self.add_mapping("dolphin".to_string(), vec![
            "dolphin".to_string(),
            "file-manager".to_string(),
            "folder".to_string(),
        ]);
        
        self.add_mapping("thunar".to_string(), vec![
            "thunar".to_string(),
            "file-manager".to_string(),
            "folder".to_string(),
        ]);

        // Media applications
        self.add_mapping("vlc".to_string(), vec![
            "vlc".to_string(),
            "vlc-media-player".to_string(),
        ]);
        
        self.add_mapping("mpv".to_string(), vec![
            "mpv".to_string(),
            "video-player".to_string(),
        ]);
        
        self.add_mapping("spotify".to_string(), vec![
            "spotify".to_string(),
            "audio-player".to_string(),
        ]);

        // Communication
        self.add_mapping("discord".to_string(), vec![
            "discord".to_string(),
            "chat".to_string(),
        ]);
        
        self.add_mapping("slack".to_string(), vec![
            "slack".to_string(),
            "chat".to_string(),
        ]);
        
        self.add_mapping("telegram-desktop".to_string(), vec![
            "telegram".to_string(),
            "telegram-desktop".to_string(),
        ]);

        // Development tools
        self.add_mapping("jetbrains-idea".to_string(), vec![
            "intellij-idea".to_string(),
            "idea".to_string(),
            "jetbrains-idea".to_string(),
        ]);
        
        self.add_mapping("jetbrains-pycharm".to_string(), vec![
            "pycharm".to_string(),
            "jetbrains-pycharm".to_string(),
        ]);
        
        self.add_mapping("DBeaver".to_string(), vec![
            "dbeaver".to_string(),
            "database".to_string(),
        ]);

        // Graphics and design
        self.add_mapping("gimp".to_string(), vec![
            "gimp".to_string(),
            "gnu-image-manipulation-program".to_string(),
        ]);
        
        self.add_mapping("inkscape".to_string(), vec![
            "inkscape".to_string(),
            "vector-graphics".to_string(),
        ]);
        
        self.add_mapping("blender".to_string(), vec![
            "blender".to_string(),
            "3d-graphics".to_string(),
        ]);

        // Office applications
        self.add_mapping("libreoffice-writer".to_string(), vec![
            "libreoffice-writer".to_string(),
            "writer".to_string(),
            "text-editor".to_string(),
        ]);
        
        self.add_mapping("libreoffice-calc".to_string(), vec![
            "libreoffice-calc".to_string(),
            "calc".to_string(),
            "spreadsheet".to_string(),
        ]);

        // System applications
        self.add_mapping("gnome-system-monitor".to_string(), vec![
            "gnome-system-monitor".to_string(),
            "system-monitor".to_string(),
            "task-manager".to_string(),
        ]);
        
        self.add_mapping("htop".to_string(), vec![
            "htop".to_string(),
            "system-monitor".to_string(),
            "task-manager".to_string(),
        ]);

        debug!("ApplicationMapper: Loaded {} default mappings", self.mapping_count());
    }

    /// Merge another mapper's mappings into this one
    pub fn merge(&mut self, other: &ApplicationMapper) {
        for (class, icon_names) in &other.class_mappings {
            // If we already have this class, extend the icon names
            if let Some(existing_names) = self.class_mappings.get_mut(class) {
                for name in icon_names {
                    if !existing_names.contains(name) {
                        existing_names.push(name.clone());
                    }
                }
            } else {
                // Add new mapping
                self.add_mapping(class.clone(), icon_names.clone());
            }
        }
    }

    /// Clear all mappings
    pub fn clear(&mut self) {
        self.class_mappings.clear();
        self.reverse_mappings.clear();
    }
}

impl Default for ApplicationMapper {
    fn default() -> Self {
        Self::new()
    }
}

/// Strategy that uses application mappings to find appropriate icon names
/// 
/// This strategy consults a configurable mapping of window classes to icon names.
/// It's particularly useful for applications that have non-standard class names
/// or need specific icon mappings.
pub struct MappingStrategy {
    /// The application mapper containing class-to-icon mappings
    mapper: Arc<RwLock<ApplicationMapper>>,
    /// Whether to use fuzzy matching for class names
    fuzzy_matching: bool,
}

impl MappingStrategy {
    /// Create a new MappingStrategy with default mappings
    pub fn new() -> Self {
        Self {
            mapper: Arc::new(RwLock::new(ApplicationMapper::new())),
            fuzzy_matching: true,
        }
    }

    /// Create a MappingStrategy with a custom mapper
    pub fn with_mapper(mapper: ApplicationMapper) -> Self {
        Self {
            mapper: Arc::new(RwLock::new(mapper)),
            fuzzy_matching: true,
        }
    }

    /// Create a MappingStrategy with custom mappings
    pub fn with_mappings(mappings: HashMap<String, Vec<String>>) -> Self {
        Self {
            mapper: Arc::new(RwLock::new(ApplicationMapper::with_custom_mappings(mappings))),
            fuzzy_matching: true,
        }
    }

    /// Enable or disable fuzzy matching
    pub fn with_fuzzy_matching(mut self, enabled: bool) -> Self {
        self.fuzzy_matching = enabled;
        self
    }

    /// Add a mapping to the strategy
    pub fn add_mapping(&self, class: String, icon_names: Vec<String>) -> Result<(), String> {
        match self.mapper.write() {
            Ok(mut mapper) => {
                mapper.add_mapping(class, icon_names);
                Ok(())
            }
            Err(e) => Err(format!("Failed to acquire write lock: {}", e)),
        }
    }

    /// Add an alias for a class
    pub fn add_alias(&self, class: String, icon_name: String) -> Result<(), String> {
        match self.mapper.write() {
            Ok(mut mapper) => {
                mapper.add_alias(class, icon_name);
                Ok(())
            }
            Err(e) => Err(format!("Failed to acquire write lock: {}", e)),
        }
    }

    /// Get the current mapper (read-only access)
    pub fn get_mapper(&self) -> Result<std::sync::RwLockReadGuard<ApplicationMapper>, String> {
        self.mapper.read().map_err(|e| format!("Failed to acquire read lock: {}", e))
    }

    /// Try to find icon names using fuzzy matching
    fn fuzzy_match_class(&self, mapper: &ApplicationMapper, class: &str) -> Option<Vec<String>> {
        if !self.fuzzy_matching {
            return None;
        }

        let class_lower = class.to_lowercase();
        
        // Try partial matches
        for (mapped_class, icon_names) in mapper.class_mappings.iter() {
            let mapped_lower = mapped_class.to_lowercase();
            
            // Check if the class contains the mapped class or vice versa
            if class_lower.contains(&mapped_lower) || mapped_lower.contains(&class_lower) {
                debug!("MappingStrategy: Fuzzy match '{}' -> '{}' -> {:?}", 
                       class, mapped_class, icon_names);
                return Some(icon_names.clone());
            }
        }

        // Try matching without common prefixes/suffixes
        let cleaned_class = class_lower
            .trim_start_matches("org.")
            .trim_start_matches("com.")
            .trim_start_matches("net.")
            .trim_end_matches(".desktop")
            .replace('-', "")
            .replace('_', "")
            .replace('.', "");

        for (mapped_class, icon_names) in mapper.class_mappings.iter() {
            let cleaned_mapped = mapped_class.to_lowercase()
                .replace('-', "")
                .replace('_', "")
                .replace('.', "");
            
            if cleaned_class == cleaned_mapped {
                debug!("MappingStrategy: Cleaned fuzzy match '{}' -> '{}' -> {:?}", 
                       class, mapped_class, icon_names);
                return Some(icon_names.clone());
            }
        }

        None
    }

    /// Generate additional icon names from context
    fn generate_context_names(&self, context: &IconContext) -> Vec<String> {
        let mut names = Vec::new();

        // Use executable name if available
        if let Some(ref executable) = context.executable {
            names.push(executable.clone());
            names.push(executable.to_lowercase());
        }

        // Extract potential application names from title
        if let Some(ref title) = context.title {
            // Common patterns in window titles
            let title_lower = title.to_lowercase();
            
            // Try splitting on common separators
            for separator in &[" - ", " – ", " | ", ": ", " ("] {
                if let Some(app_part) = title_lower.split(separator).next() {
                    let app_name = app_part.trim().to_string();
                    if !app_name.is_empty() && app_name.len() > 2 {
                        names.push(app_name);
                    }
                }
            }
        }

        names
    }
}

impl IconDetectionStrategy for MappingStrategy {
    fn detect_icon(&self, context: &IconContext) -> Option<IconResult> {
        let mapper = match self.mapper.read() {
            Ok(mapper) => mapper,
            Err(e) => {
                warn!("MappingStrategy: Failed to acquire read lock: {}", e);
                return None;
            }
        };

        debug!("MappingStrategy: Looking for mappings for class '{}'", context.class);

        // Try exact mapping first
        if let Some(icon_names) = mapper.get_icon_names(&context.class) {
            debug!("MappingStrategy: Found exact mapping for '{}': {:?}", 
                   context.class, icon_names);
            
            // Return the first icon name as a result
            // The actual icon file resolution will be handled by other strategies
            if let Some(first_name) = icon_names.first() {
                let metadata = IconMetadata::new(IconFormat::Other("mapped".to_string()));
                
                return Some(IconResult::new(
                    std::path::PathBuf::from(first_name), // This will be resolved by other strategies
                    "MappingStrategy".to_string(),
                    0.9, // High confidence - we have an explicit mapping
                    metadata,
                ));
            }
        }

        // Try fuzzy matching
        if let Some(icon_names) = self.fuzzy_match_class(&mapper, &context.class) {
            if let Some(first_name) = icon_names.first() {
                let metadata = IconMetadata::new(IconFormat::Other("mapped".to_string()));
                
                return Some(IconResult::new(
                    std::path::PathBuf::from(first_name),
                    "MappingStrategy".to_string(),
                    0.7, // Medium-high confidence - fuzzy match
                    metadata,
                ));
            }
        }

        // Try context-based names
        let context_names = self.generate_context_names(context);
        for context_name in context_names {
            if let Some(icon_names) = mapper.get_icon_names(&context_name) {
                debug!("MappingStrategy: Found context-based mapping '{}' -> {:?}", 
                       context_name, icon_names);
                
                if let Some(first_name) = icon_names.first() {
                    let metadata = IconMetadata::new(IconFormat::Other("mapped".to_string()));
                    
                    return Some(IconResult::new(
                        std::path::PathBuf::from(first_name),
                        "MappingStrategy".to_string(),
                        0.6, // Medium confidence - context-based match
                        metadata,
                    ));
                }
            }
        }

        debug!("MappingStrategy: No mapping found for class '{}'", context.class);
        None
    }

    fn priority(&self) -> u8 {
        60 // High priority - explicit mappings should be preferred
    }

    fn name(&self) -> &'static str {
        "MappingStrategy"
    }

    fn is_available(&self) -> bool {
        // Available if we have any mappings
        self.mapper.read()
            .map(|mapper| mapper.mapping_count() > 0)
            .unwrap_or(false)
    }

    fn initialize(&mut self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let mapper = self.mapper.read()
            .map_err(|e| format!("Failed to acquire read lock: {}", e))?;
        
        debug!("MappingStrategy: Initialized with {} mappings", mapper.mapping_count());
        Ok(())
    }

    fn cleanup(&mut self) {
        debug!("MappingStrategy: Cleanup completed");
    }
}

impl Default for MappingStrategy {
    fn default() -> Self {
        Self::new()
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn test_application_mapper_creation() {
        let mapper = ApplicationMapper::new();
        assert!(mapper.mapping_count() > 0); // Should have default mappings
        assert!(mapper.has_mapping("firefox"));
        assert!(mapper.has_mapping("Google-chrome"));
    }

    #[test]
    fn test_application_mapper_custom_mappings() {
        let mut custom_mappings = HashMap::new();
        custom_mappings.insert("test-app".to_string(), vec!["test-icon".to_string()]);
        
        let mapper = ApplicationMapper::with_custom_mappings(custom_mappings);
        assert_eq!(mapper.mapping_count(), 1);
        assert!(mapper.has_mapping("test-app"));
        assert!(!mapper.has_mapping("firefox")); // Should not have defaults
    }

    #[test]
    fn test_add_mapping() {
        let mut mapper = ApplicationMapper::new();
        let initial_count = mapper.mapping_count();
        
        mapper.add_mapping("new-app".to_string(), vec!["new-icon".to_string(), "alt-icon".to_string()]);
        
        assert_eq!(mapper.mapping_count(), initial_count + 1);
        assert!(mapper.has_mapping("new-app"));
        
        let icon_names = mapper.get_icon_names("new-app").unwrap();
        assert_eq!(icon_names.len(), 2);
        assert_eq!(icon_names[0], "new-icon");
        assert_eq!(icon_names[1], "alt-icon");
    }

    #[test]
    fn test_add_alias() {
        let mut mapper = ApplicationMapper::new();
        
        mapper.add_alias("firefox".to_string(), "firefox-esr".to_string());
        
        let icon_names = mapper.get_icon_names("firefox").unwrap();
        assert!(icon_names.contains(&"firefox-esr".to_string()));
    }

    #[test]
    fn test_get_icon_names_case_insensitive() {
        let mapper = ApplicationMapper::new();
        
        // Test exact match
        assert!(mapper.get_icon_names("firefox").is_some());
        
        // Test case-insensitive match
        assert!(mapper.get_icon_names("FIREFOX").is_some());
        assert!(mapper.get_icon_names("Firefox").is_some());
    }

    #[test]
    fn test_reverse_mapping() {
        let mapper = ApplicationMapper::new();
        
        // Should have reverse mapping for preferred icon names
        assert!(mapper.get_preferred_class("firefox").is_some());
        assert!(mapper.get_preferred_class("google-chrome").is_some());
    }

    #[test]
    fn test_empty_icon_names() {
        let mut mapper = ApplicationMapper::new();
        let initial_count = mapper.mapping_count();
        
        // Adding empty icon names should be ignored
        mapper.add_mapping("empty-app".to_string(), vec![]);
        
        assert_eq!(mapper.mapping_count(), initial_count);
        assert!(!mapper.has_mapping("empty-app"));
    }

    #[test]
    fn test_merge_mappers() {
        let mut mapper1 = ApplicationMapper::new();
        let mut mapper2 = ApplicationMapper::new();
        
        // Add unique mapping to mapper2
        mapper2.add_mapping("unique-app".to_string(), vec!["unique-icon".to_string()]);
        
        // Add additional alias to existing app in mapper2
        mapper2.add_alias("firefox".to_string(), "firefox-custom".to_string());
        
        let initial_count = mapper1.mapping_count();
        mapper1.merge(&mapper2);
        
        // Should have one more mapping
        assert_eq!(mapper1.mapping_count(), initial_count + 1);
        
        // Should have the unique mapping
        assert!(mapper1.has_mapping("unique-app"));
        
        // Should have the additional alias for firefox
        let firefox_names = mapper1.get_icon_names("firefox").unwrap();
        assert!(firefox_names.contains(&"firefox-custom".to_string()));
    }

    #[test]
    fn test_clear_mappings() {
        let mut mapper = ApplicationMapper::new();
        assert!(mapper.mapping_count() > 0);
        
        mapper.clear();
        assert_eq!(mapper.mapping_count(), 0);
        assert!(!mapper.has_mapping("firefox"));
    }

    #[test]
    fn test_mapping_strategy_creation() {
        let strategy = MappingStrategy::new();
        assert_eq!(strategy.name(), "MappingStrategy");
        assert_eq!(strategy.priority(), 60);
        assert!(strategy.is_available());
    }

    #[test]
    fn test_mapping_strategy_with_custom_mappings() {
        let mut mappings = HashMap::new();
        mappings.insert("test-class".to_string(), vec!["test-icon".to_string()]);
        
        let strategy = MappingStrategy::with_mappings(mappings);
        assert!(strategy.is_available());
        
        let mapper = strategy.get_mapper().unwrap();
        assert!(mapper.has_mapping("test-class"));
    }

    #[test]
    fn test_mapping_strategy_exact_match() {
        let strategy = MappingStrategy::new();
        
        let context = IconContext::new("firefox".to_string());
        let result = strategy.detect_icon(&context);
        
        assert!(result.is_some());
        let result = result.unwrap();
        assert_eq!(result.strategy_used, "MappingStrategy");
        assert_eq!(result.confidence, 0.9); // High confidence for exact match
        assert_eq!(result.path.to_string_lossy(), "firefox");
    }

    #[test]
    fn test_mapping_strategy_case_insensitive_match() {
        let strategy = MappingStrategy::new();
        
        let context = IconContext::new("FIREFOX".to_string());
        let result = strategy.detect_icon(&context);
        
        assert!(result.is_some());
        let result = result.unwrap();
        assert_eq!(result.strategy_used, "MappingStrategy");
        assert_eq!(result.confidence, 0.9);
    }

    #[test]
    fn test_mapping_strategy_fuzzy_matching() {
        let strategy = MappingStrategy::new();
        
        // Test partial match
        let context = IconContext::new("firefox-esr".to_string());
        let result = strategy.detect_icon(&context);
        
        assert!(result.is_some());
        let result = result.unwrap();
        assert_eq!(result.strategy_used, "MappingStrategy");
        assert_eq!(result.confidence, 0.7); // Medium-high confidence for fuzzy match
    }

    #[test]
    fn test_mapping_strategy_fuzzy_matching_disabled() {
        let strategy = MappingStrategy::new().with_fuzzy_matching(false);
        
        // Should not find fuzzy matches when disabled
        let context = IconContext::new("firefox-esr".to_string());
        let result = strategy.detect_icon(&context);
        
        assert!(result.is_none());
    }

    #[test]
    fn test_mapping_strategy_context_based_matching() {
        let strategy = MappingStrategy::new();
        
        // Test matching based on executable name
        let context = IconContext::new("unknown-class".to_string())
            .with_executable("firefox".to_string());
        let result = strategy.detect_icon(&context);
        
        assert!(result.is_some());
        let result = result.unwrap();
        assert_eq!(result.strategy_used, "MappingStrategy");
        assert_eq!(result.confidence, 0.6); // Medium confidence for context match
    }

    #[test]
    fn test_mapping_strategy_title_based_matching() {
        let strategy = MappingStrategy::new();
        
        // Test matching based on window title
        let context = IconContext::with_title("unknown-class".to_string(), "Firefox - Mozilla Firefox".to_string());
        let result = strategy.detect_icon(&context);
        
        assert!(result.is_some());
        let result = result.unwrap();
        assert_eq!(result.strategy_used, "MappingStrategy");
        assert_eq!(result.confidence, 0.6);
    }

    #[test]
    fn test_mapping_strategy_no_match() {
        let strategy = MappingStrategy::new();
        
        let context = IconContext::new("completely-unknown-app".to_string());
        let result = strategy.detect_icon(&context);
        
        assert!(result.is_none());
    }

    #[test]
    fn test_mapping_strategy_add_mapping() {
        let strategy = MappingStrategy::new();
        
        // Add a new mapping
        let result = strategy.add_mapping(
            "new-app".to_string(), 
            vec!["new-icon".to_string()]
        );
        assert!(result.is_ok());
        
        // Test that the new mapping works
        let context = IconContext::new("new-app".to_string());
        let result = strategy.detect_icon(&context);
        
        assert!(result.is_some());
        let result = result.unwrap();
        assert_eq!(result.path.to_string_lossy(), "new-icon");
    }

    #[test]
    fn test_mapping_strategy_add_alias() {
        let strategy = MappingStrategy::new();
        
        // Add an alias to existing mapping
        let result = strategy.add_alias("firefox".to_string(), "firefox-test".to_string());
        assert!(result.is_ok());
        
        // Verify the alias was added
        let mapper = strategy.get_mapper().unwrap();
        let icon_names = mapper.get_icon_names("firefox").unwrap();
        assert!(icon_names.contains(&"firefox-test".to_string()));
    }

    #[test]
    fn test_default_mappings_coverage() {
        let mapper = ApplicationMapper::new();
        
        // Test that we have mappings for common application categories
        
        // Web browsers
        assert!(mapper.has_mapping("firefox"));
        assert!(mapper.has_mapping("Google-chrome"));
        assert!(mapper.has_mapping("chromium-browser"));
        
        // Text editors
        assert!(mapper.has_mapping("code"));
        assert!(mapper.has_mapping("nvim"));
        
        // Terminals
        assert!(mapper.has_mapping("Alacritty"));
        assert!(mapper.has_mapping("kitty"));
        
        // File managers
        assert!(mapper.has_mapping("nautilus"));
        assert!(mapper.has_mapping("dolphin"));
        
        // Media applications
        assert!(mapper.has_mapping("vlc"));
        assert!(mapper.has_mapping("spotify"));
        
        // Communication
        assert!(mapper.has_mapping("discord"));
        assert!(mapper.has_mapping("slack"));
        
        // Development tools
        assert!(mapper.has_mapping("jetbrains-idea"));
        assert!(mapper.has_mapping("DBeaver"));
        
        // Graphics
        assert!(mapper.has_mapping("gimp"));
        assert!(mapper.has_mapping("inkscape"));
        
        // Office
        assert!(mapper.has_mapping("libreoffice-writer"));
        
        // System
        assert!(mapper.has_mapping("gnome-system-monitor"));
    }

    #[test]
    fn test_multiple_aliases_per_application() {
        let mapper = ApplicationMapper::new();
        
        // Test that applications have multiple aliases
        let firefox_names = mapper.get_icon_names("firefox").unwrap();
        assert!(firefox_names.len() > 1);
        assert!(firefox_names.contains(&"firefox".to_string()));
        assert!(firefox_names.contains(&"Firefox".to_string()));
        
        let chrome_names = mapper.get_icon_names("Google-chrome").unwrap();
        assert!(chrome_names.len() > 1);
        assert!(chrome_names.contains(&"google-chrome".to_string()));
        assert!(chrome_names.contains(&"chrome".to_string()));
    }

    #[test]
    fn test_fuzzy_matching_with_prefixes() {
        let strategy = MappingStrategy::new();
        
        // Test matching with common prefixes
        let context = IconContext::new("org.mozilla.firefox".to_string());
        let result = strategy.detect_icon(&context);
        
        assert!(result.is_some());
        let result = result.unwrap();
        assert_eq!(result.strategy_used, "MappingStrategy");
    }

    #[test]
    fn test_fuzzy_matching_with_separators() {
        let strategy = MappingStrategy::new();
        
        // Test matching with different separators
        let context = IconContext::new("google_chrome".to_string());
        let result = strategy.detect_icon(&context);
        
        assert!(result.is_some());
        let result = result.unwrap();
        assert_eq!(result.strategy_used, "MappingStrategy");
    }

    #[test]
    fn test_generate_context_names() {
        let strategy = MappingStrategy::new();
        
        let context = IconContext::with_title("unknown".to_string(), "Mozilla Firefox - Private Browsing".to_string())
            .with_executable("firefox".to_string());
        
        let names = strategy.generate_context_names(&context);
        
        assert!(names.contains(&"firefox".to_string()));
        assert!(names.contains(&"mozilla firefox".to_string()));
    }

    #[test]
    fn test_strategy_initialization() {
        let mut strategy = MappingStrategy::new();
        let result = strategy.initialize();
        
        assert!(result.is_ok());
    }

    #[test]
    fn test_strategy_availability() {
        // Strategy with mappings should be available
        let strategy = MappingStrategy::new();
        assert!(strategy.is_available());
        
        // Strategy with no mappings should not be available
        let empty_mappings = HashMap::new();
        let empty_strategy = MappingStrategy::with_mappings(empty_mappings);
        assert!(!empty_strategy.is_available());
    }

    #[test]
    fn test_confidence_levels() {
        let strategy = MappingStrategy::new();
        
        // Exact match should have highest confidence
        let exact_context = IconContext::new("firefox".to_string());
        let exact_result = strategy.detect_icon(&exact_context).unwrap();
        assert_eq!(exact_result.confidence, 0.9);
        
        // Fuzzy match should have medium-high confidence
        let fuzzy_context = IconContext::new("firefox-esr".to_string());
        let fuzzy_result = strategy.detect_icon(&fuzzy_context).unwrap();
        assert_eq!(fuzzy_result.confidence, 0.7);
        
        // Context match should have medium confidence
        let context_match = IconContext::new("unknown".to_string())
            .with_executable("firefox".to_string());
        let context_result = strategy.detect_icon(&context_match).unwrap();
        assert_eq!(context_result.confidence, 0.6);
    }

    #[test]
    fn test_icon_format_in_result() {
        let strategy = MappingStrategy::new();
        
        let context = IconContext::new("firefox".to_string());
        let result = strategy.detect_icon(&context).unwrap();
        
        // MappingStrategy should return "mapped" format
        assert!(matches!(result.metadata.format, IconFormat::Other(ref s) if s == "mapped"));
    }
}