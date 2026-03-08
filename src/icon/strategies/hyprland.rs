use crate::icon::traits::IconDetectionStrategy;
use crate::icon::types::{IconContext, IconResult, IconMetadata, IconFormat};
use std::path::PathBuf;
use std::process::Command;
use std::collections::HashMap;
use regex::Regex;
use bevy::log::{debug, info};

/// Strategy that uses enhanced Hyprland IPC information for icon detection
/// 
/// This strategy leverages additional information available from Hyprland IPC
/// to improve icon detection by:
/// 1. Parsing window titles to extract executable names
/// 2. Using process information to map to icon names
/// 3. Extracting application names from various title formats
pub struct HyprlandStrategy {
    /// Regex patterns for extracting executable names from window titles
    title_patterns: Vec<TitlePattern>,
    /// Cache for process ID to executable name mappings
    pid_cache: HashMap<u32, String>,
    /// Common application name mappings from titles
    title_mappings: HashMap<String, String>,
}

/// Pattern for extracting information from window titles
struct TitlePattern {
    /// Regex pattern to match against titles
    regex: Regex,
    /// Group index that contains the executable/application name
    name_group: usize,
    /// Description of what this pattern matches
    description: &'static str,
}

impl HyprlandStrategy {
    /// Create a new HyprlandStrategy with default patterns
    pub fn new() -> Self {
        let mut strategy = Self {
            title_patterns: Vec::new(),
            pid_cache: HashMap::new(),
            title_mappings: HashMap::new(),
        };
        
        strategy.initialize_patterns();
        strategy.initialize_mappings();
        strategy
    }

    /// Initialize regex patterns for extracting names from window titles
    fn initialize_patterns(&mut self) {
        // Pattern for "AppName - Document" format (e.g., "Firefox - Mozilla Firefox")
        if let Ok(regex) = Regex::new(r"^([^-]+)\s*-\s*.+$") {
            self.title_patterns.push(TitlePattern {
                regex,
                name_group: 1,
                description: "AppName - Document format",
            });
        }

        // Pattern for "Document - AppName" format (e.g., "index.html - Visual Studio Code")
        if let Ok(regex) = Regex::new(r"^.+\s*-\s*([^-]+)$") {
            self.title_patterns.push(TitlePattern {
                regex,
                name_group: 1,
                description: "Document - AppName format",
            });
        }

        // Pattern for "[AppName]" format (e.g., "[Spotify]")
        if let Ok(regex) = Regex::new(r"^\[([^\]]+)\]") {
            self.title_patterns.push(TitlePattern {
                regex,
                name_group: 1,
                description: "Bracketed app name format",
            });
        }

        // Pattern for "AppName:" format (e.g., "Discord:")
        if let Ok(regex) = Regex::new(r"^([^:]+):") {
            self.title_patterns.push(TitlePattern {
                regex,
                name_group: 1,
                description: "AppName: format",
            });
        }

        // Pattern for executable in parentheses (e.g., "Window Title (firefox)")
        if let Ok(regex) = Regex::new(r"\(([^)]+)\)$") {
            self.title_patterns.push(TitlePattern {
                regex,
                name_group: 1,
                description: "Executable in parentheses",
            });
        }

        // Pattern for version numbers (e.g., "Firefox 120.0")
        if let Ok(regex) = Regex::new(r"^([A-Za-z]+)\s+\d+") {
            self.title_patterns.push(TitlePattern {
                regex,
                name_group: 1,
                description: "AppName with version",
            });
        }
    }

    /// Initialize common title to application name mappings
    fn initialize_mappings(&mut self) {
        // Common application title variations
        self.title_mappings.insert("Mozilla Firefox".to_string(), "firefox".to_string());
        self.title_mappings.insert("Firefox".to_string(), "firefox".to_string());
        self.title_mappings.insert("Google Chrome".to_string(), "google-chrome".to_string());
        self.title_mappings.insert("Chrome".to_string(), "google-chrome".to_string());
        self.title_mappings.insert("Chromium".to_string(), "chromium".to_string());
        self.title_mappings.insert("Visual Studio Code".to_string(), "code".to_string());
        self.title_mappings.insert("VS Code".to_string(), "code".to_string());
        self.title_mappings.insert("Code".to_string(), "code".to_string());
        self.title_mappings.insert("Spotify".to_string(), "spotify".to_string());
        self.title_mappings.insert("Discord".to_string(), "discord".to_string());
        self.title_mappings.insert("Telegram".to_string(), "telegram".to_string());
        self.title_mappings.insert("WhatsApp".to_string(), "whatsapp".to_string());
        self.title_mappings.insert("Slack".to_string(), "slack".to_string());
        self.title_mappings.insert("GIMP".to_string(), "gimp".to_string());
        self.title_mappings.insert("LibreOffice".to_string(), "libreoffice".to_string());
        self.title_mappings.insert("Thunderbird".to_string(), "thunderbird".to_string());
        self.title_mappings.insert("VLC".to_string(), "vlc".to_string());
        self.title_mappings.insert("Nautilus".to_string(), "nautilus".to_string());
        self.title_mappings.insert("Files".to_string(), "nautilus".to_string());
        self.title_mappings.insert("Terminal".to_string(), "gnome-terminal".to_string());
        self.title_mappings.insert("Konsole".to_string(), "konsole".to_string());
        self.title_mappings.insert("Alacritty".to_string(), "alacritty".to_string());
        self.title_mappings.insert("Kitty".to_string(), "kitty".to_string());
    }

    /// Extract potential application names from window title
    fn extract_names_from_title(&self, title: &str) -> Vec<String> {
        let mut names = Vec::new();
        
        debug!("Extracting names from title: '{}'", title);

        // First, check direct mappings
        if let Some(mapped_name) = self.title_mappings.get(title) {
            names.push(mapped_name.clone());
            debug!("Found direct mapping: '{}' -> '{}'", title, mapped_name);
        }

        // Try each regex pattern
        for pattern in &self.title_patterns {
            if let Some(captures) = pattern.regex.captures(title) {
                if let Some(matched) = captures.get(pattern.name_group) {
                    let extracted = matched.as_str().trim().to_string();
                    if !extracted.is_empty() && !names.contains(&extracted) {
                        names.push(extracted.clone());
                        debug!("Pattern '{}' extracted: '{}'", pattern.description, extracted);
                        
                        // Also try lowercase version
                        let lowercase = extracted.to_lowercase();
                        if !names.contains(&lowercase) {
                            names.push(lowercase);
                        }

                        // Check if extracted name has a mapping
                        if let Some(mapped) = self.title_mappings.get(&extracted) {
                            if !names.contains(mapped) {
                                names.push(mapped.clone());
                                debug!("Mapped extracted name: '{}' -> '{}'", extracted, mapped);
                            }
                        }
                    }
                }
            }
        }

        // Split title by common separators and try each part
        for separator in &[" - ", " | ", " :: ", " — "] {
            if title.contains(separator) {
                for part in title.split(separator) {
                    let trimmed = part.trim();
                    if !trimmed.is_empty() && trimmed.len() > 2 {
                        // Check if this part has a mapping
                        if let Some(mapped) = self.title_mappings.get(trimmed) {
                            if !names.contains(mapped) {
                                names.push(mapped.clone());
                                debug!("Found mapping for title part: '{}' -> '{}'", trimmed, mapped);
                            }
                        }
                        
                        // Add the part itself (lowercase)
                        let lowercase_part = trimmed.to_lowercase();
                        if !names.contains(&lowercase_part) {
                            names.push(lowercase_part);
                        }
                    }
                }
            }
        }

        debug!("Extracted {} names from title: {:?}", names.len(), names);
        names
    }

    /// Get executable name from process ID using /proc filesystem
    fn get_executable_from_pid(&mut self, pid: u32) -> Option<String> {
        // Check cache first
        if let Some(cached) = self.pid_cache.get(&pid) {
            return Some(cached.clone());
        }

        // Try to read from /proc/PID/comm (process name)
        let comm_path = format!("/proc/{}/comm", pid);
        if let Ok(comm) = std::fs::read_to_string(&comm_path) {
            let executable = comm.trim().to_string();
            if !executable.is_empty() {
                debug!("Found executable from /proc/{}/comm: '{}'", pid, executable);
                self.pid_cache.insert(pid, executable.clone());
                return Some(executable);
            }
        }

        // Try to read from /proc/PID/cmdline (command line)
        let cmdline_path = format!("/proc/{}/cmdline", pid);
        if let Ok(cmdline) = std::fs::read_to_string(&cmdline_path) {
            // cmdline is null-separated, take the first part (executable path)
            if let Some(first_arg) = cmdline.split('\0').next() {
                if let Some(executable) = std::path::Path::new(first_arg).file_name() {
                    if let Some(name) = executable.to_str() {
                        let name = name.to_string();
                        debug!("Found executable from /proc/{}/cmdline: '{}'", pid, name);
                        self.pid_cache.insert(pid, name.clone());
                        return Some(name);
                    }
                }
            }
        }

        // Try using ps command as fallback
        if let Ok(output) = Command::new("ps")
            .args(&["-p", &pid.to_string(), "-o", "comm="])
            .output()
        {
            if output.status.success() {
                let comm = String::from_utf8_lossy(&output.stdout).trim().to_string();
                if !comm.is_empty() {
                    debug!("Found executable from ps command: '{}'", comm);
                    self.pid_cache.insert(pid, comm.clone());
                    return Some(comm);
                }
            }
        }

        debug!("Could not determine executable for PID {}", pid);
        None
    }

    /// Try to find icon using Hyprland-specific information
    fn find_icon_with_hyprland_info(&mut self, context: &IconContext) -> Option<IconResult> {
        let mut candidate_names = Vec::new();

        // If we have a PID, try to get the executable name
        if let Some(pid) = context.pid {
            if let Some(executable) = self.get_executable_from_pid(pid) {
                candidate_names.push(executable.clone());
                candidate_names.push(executable.to_lowercase());
                debug!("Added executable from PID {}: '{}'", pid, executable);
            }
        }

        // If we have a title, extract potential names from it
        if let Some(ref title) = context.title {
            let title_names = self.extract_names_from_title(title);
            for name in title_names {
                if !candidate_names.contains(&name) {
                    candidate_names.push(name);
                }
            }
        }

        // Try to find icons for each candidate name
        for name in &candidate_names {
            if let Some(icon_path) = self.find_icon_for_name(name) {
                let format = IconFormat::from_extension(
                    icon_path.extension()
                        .and_then(|ext| ext.to_str())
                        .unwrap_or("png")
                );
                
                let metadata = IconMetadata::new(format);
                
                info!("HyprlandStrategy found icon for '{}': {:?}", name, icon_path);
                
                return Some(IconResult::new(
                    icon_path,
                    "HyprlandStrategy".to_string(),
                    0.8, // High confidence since we used Hyprland-specific info
                    metadata,
                ));
            }
        }

        debug!("HyprlandStrategy could not find icon for any candidate names: {:?}", candidate_names);
        None
    }

    /// Find icon file for a given application name
    fn find_icon_for_name(&self, name: &str) -> Option<PathBuf> {
        // Standard icon directories to search
        let icon_dirs = [
            "/usr/share/icons/hicolor/48x48/apps",
            "/usr/share/icons/hicolor/scalable/apps",
            "/usr/share/pixmaps",
            "/usr/share/icons",
            "/usr/local/share/icons",
        ];

        // Icon file extensions to try
        let extensions = ["svg", "png", "xpm"];

        for dir in &icon_dirs {
            for ext in &extensions {
                let icon_path = PathBuf::from(dir).join(format!("{}.{}", name, ext));
                if icon_path.exists() {
                    debug!("Found icon at: {:?}", icon_path);
                    return Some(icon_path);
                }
            }
        }

        // Try with common variations
        let variations = [
            format!("{}-icon", name),
            format!("application-{}", name),
            format!("{}_icon", name),
            name.replace("-", "_"),
            name.replace("_", "-"),
        ];

        for dir in &icon_dirs {
            for variation in &variations {
                for ext in &extensions {
                    let icon_path = PathBuf::from(dir).join(format!("{}.{}", variation, ext));
                    if icon_path.exists() {
                        debug!("Found icon with variation '{}' at: {:?}", variation, icon_path);
                        return Some(icon_path);
                    }
                }
            }
        }

        None
    }
}

impl Default for HyprlandStrategy {
    fn default() -> Self {
        Self::new()
    }
}

impl IconDetectionStrategy for HyprlandStrategy {
    fn detect_icon(&self, context: &IconContext) -> Option<IconResult> {
        debug!("HyprlandStrategy attempting detection for context: {:?}", context);

        // Create a mutable copy to work with caches
        let mut strategy = HyprlandStrategy {
            title_patterns: self.title_patterns.clone(),
            pid_cache: self.pid_cache.clone(),
            title_mappings: self.title_mappings.clone(),
        };

        // Only proceed if we have additional Hyprland information beyond just the class
        let has_hyprland_info = context.title.is_some() || 
                               context.pid.is_some() || 
                               context.executable.is_some();

        if !has_hyprland_info {
            debug!("HyprlandStrategy skipping - no additional Hyprland info available");
            return None;
        }

        strategy.find_icon_with_hyprland_info(context)
    }

    fn priority(&self) -> u8 {
        75 // High priority since this uses Hyprland-specific information
    }

    fn name(&self) -> &'static str {
        "HyprlandStrategy"
    }

    fn is_available(&self) -> bool {
        // Check if we're running under Hyprland
        std::env::var("HYPRLAND_INSTANCE_SIGNATURE").is_ok()
    }
}

// Helper trait to make TitlePattern cloneable
impl Clone for TitlePattern {
    fn clone(&self) -> Self {
        Self {
            regex: Regex::new(self.regex.as_str()).unwrap(),
            name_group: self.name_group,
            description: self.description,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::icon::types::IconContext;

    #[test]
    fn test_extract_names_from_title() {
        let strategy = HyprlandStrategy::new();

        // Test "AppName - Document" format
        let names = strategy.extract_names_from_title("Firefox - Mozilla Firefox");
        assert!(names.contains(&"firefox".to_string()));

        // Test direct mapping
        let names = strategy.extract_names_from_title("Visual Studio Code");
        assert!(names.contains(&"code".to_string()));

        // Test bracketed format
        let names = strategy.extract_names_from_title("[Spotify] - Song Name");
        assert!(names.contains(&"spotify".to_string()));

        // Test version format
        let names = strategy.extract_names_from_title("Firefox 120.0");
        assert!(names.contains(&"firefox".to_string()));
    }

    #[test]
    fn test_title_mappings() {
        let strategy = HyprlandStrategy::new();
        
        // Test that common applications are mapped correctly
        assert_eq!(strategy.title_mappings.get("Mozilla Firefox"), Some(&"firefox".to_string()));
        assert_eq!(strategy.title_mappings.get("Visual Studio Code"), Some(&"code".to_string()));
        assert_eq!(strategy.title_mappings.get("Google Chrome"), Some(&"google-chrome".to_string()));
    }

    #[test]
    fn test_strategy_priority() {
        let strategy = HyprlandStrategy::new();
        assert_eq!(strategy.priority(), 75);
    }

    #[test]
    fn test_strategy_name() {
        let strategy = HyprlandStrategy::new();
        assert_eq!(strategy.name(), "HyprlandStrategy");
    }

    #[test]
    fn test_detect_icon_with_title() {
        let strategy = HyprlandStrategy::new();
        
        let context = IconContext::with_title("unknown-class".to_string(), "Firefox - Mozilla Firefox".to_string());
        
        // This test will only pass if Firefox icon actually exists on the system
        // In a real test environment, we might want to mock the file system
        let result = strategy.detect_icon(&context);
        
        // We can at least verify that the strategy processes the context
        // without panicking and returns the expected type
        match result {
            Some(icon_result) => {
                assert_eq!(icon_result.strategy_used, "HyprlandStrategy");
                assert_eq!(icon_result.confidence, 0.8);
            }
            None => {
                // This is expected if the icon doesn't exist on the test system
            }
        }
    }

    #[test]
    fn test_detect_icon_without_hyprland_info() {
        let strategy = HyprlandStrategy::new();
        
        // Context with only class (no Hyprland-specific info)
        let context = IconContext::new("firefox".to_string());
        
        // Should return None since there's no additional Hyprland info
        let result = strategy.detect_icon(&context);
        assert!(result.is_none());
    }

    #[test]
    fn test_detect_icon_with_executable() {
        let strategy = HyprlandStrategy::new();
        
        let context = IconContext::new("unknown-class".to_string())
            .with_executable("firefox".to_string());
        
        // Should attempt detection since we have executable info
        let result = strategy.detect_icon(&context);
        
        // Result depends on whether icon exists, but should not panic
        match result {
            Some(icon_result) => {
                assert_eq!(icon_result.strategy_used, "HyprlandStrategy");
            }
            None => {
                // Expected if icon doesn't exist
            }
        }
    }
}