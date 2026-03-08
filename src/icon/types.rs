use std::path::PathBuf;
use std::time::Instant;

/// Context information available for icon detection
#[derive(Debug, Clone)]
pub struct IconContext {
    /// Window class from Hyprland
    pub class: String,
    /// Window title from Hyprland (optional)
    pub title: Option<String>,
    /// Executable name if available
    pub executable: Option<String>,
    /// Process ID if available
    pub pid: Option<u32>,
    /// Workspace information if available
    pub workspace: Option<String>,
}

impl IconContext {
    /// Create a new IconContext with just the class
    pub fn new(class: String) -> Self {
        Self {
            class,
            title: None,
            executable: None,
            pid: None,
            workspace: None,
        }
    }

    /// Create a new IconContext with class and title
    pub fn with_title(class: String, title: String) -> Self {
        Self {
            class,
            title: Some(title),
            executable: None,
            pid: None,
            workspace: None,
        }
    }

    /// Add executable information to the context
    pub fn with_executable(mut self, executable: String) -> Self {
        self.executable = Some(executable);
        self
    }

    /// Add process ID to the context
    pub fn with_pid(mut self, pid: u32) -> Self {
        self.pid = Some(pid);
        self
    }

    /// Add workspace information to the context
    pub fn with_workspace(mut self, workspace: String) -> Self {
        self.workspace = Some(workspace);
        self
    }
}

/// Result of icon detection containing path and metadata
#[derive(Debug, Clone)]
pub struct IconResult {
    /// Path to the icon file
    pub path: PathBuf,
    /// Name of the strategy that found this icon
    pub strategy_used: String,
    /// Confidence level of this result (0.0 - 1.0)
    pub confidence: f32,
    /// Additional metadata about the icon
    pub metadata: IconMetadata,
}

impl IconResult {
    /// Create a new IconResult
    pub fn new(
        path: PathBuf,
        strategy_used: String,
        confidence: f32,
        metadata: IconMetadata,
    ) -> Self {
        Self {
            path,
            strategy_used,
            confidence,
            metadata,
        }
    }
}

/// Metadata about an icon file
#[derive(Debug, Clone)]
pub struct IconMetadata {
    /// Format of the icon file
    pub format: IconFormat,
    /// Size of the icon if known
    pub size: Option<(u32, u32)>,
    /// Icon theme if applicable
    pub theme: Option<String>,
    /// When this metadata was created
    pub created_at: Instant,
}

impl IconMetadata {
    /// Create new metadata with format
    pub fn new(format: IconFormat) -> Self {
        Self {
            format,
            size: None,
            theme: None,
            created_at: Instant::now(),
        }
    }

    /// Create metadata with format and size
    pub fn with_size(format: IconFormat, size: (u32, u32)) -> Self {
        Self {
            format,
            size: Some(size),
            theme: None,
            created_at: Instant::now(),
        }
    }

    /// Create metadata with format and theme
    pub fn with_theme(format: IconFormat, theme: String) -> Self {
        Self {
            format,
            size: None,
            theme: Some(theme),
            created_at: Instant::now(),
        }
    }

    /// Create metadata with all information
    pub fn complete(format: IconFormat, size: (u32, u32), theme: String) -> Self {
        Self {
            format,
            size: Some(size),
            theme: Some(theme),
            created_at: Instant::now(),
        }
    }
}

/// Supported icon formats
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IconFormat {
    /// SVG vector format
    Svg,
    /// PNG raster format
    Png,
    /// XPM format
    Xpm,
    /// Other format with extension
    Other(String),
}

impl IconFormat {
    /// Create IconFormat from file extension
    pub fn from_extension(ext: &str) -> Self {
        match ext.to_lowercase().as_str() {
            "svg" => IconFormat::Svg,
            "png" => IconFormat::Png,
            "xpm" => IconFormat::Xpm,
            other => IconFormat::Other(other.to_string()),
        }
    }

    /// Get the file extension for this format
    pub fn extension(&self) -> &str {
        match self {
            IconFormat::Svg => "svg",
            IconFormat::Png => "png",
            IconFormat::Xpm => "xpm",
            IconFormat::Other(ext) => ext,
        }
    }

    /// Check if this is a vector format
    pub fn is_vector(&self) -> bool {
        matches!(self, IconFormat::Svg)
    }

    /// Check if this is a raster format
    pub fn is_raster(&self) -> bool {
        matches!(self, IconFormat::Png | IconFormat::Xpm | IconFormat::Other(_))
    }
}

/// Error types for icon operations
#[derive(Debug, thiserror::Error)]
pub enum IconError {
    #[error("No icon found for class: {class}")]
    NotFound { class: String },
    
    #[error("Failed to load icon from path: {path}")]
    LoadError { 
        path: PathBuf, 
        #[source]
        source: Box<dyn std::error::Error + Send + Sync>
    },
    
    #[error("Invalid icon format: {format}")]
    InvalidFormat { format: String },
    
    #[error("Cache error: {message}")]
    CacheError { message: String },
    
    #[error("Strategy error in {strategy}: {message}")]
    StrategyError { strategy: String, message: String },
    
    #[error("Configuration error: {message}")]
    ConfigError { message: String },
}

impl IconError {
    /// Create a NotFound error
    pub fn not_found(class: impl Into<String>) -> Self {
        Self::NotFound { class: class.into() }
    }

    /// Create a LoadError
    pub fn load_error(path: PathBuf, source: Box<dyn std::error::Error + Send + Sync>) -> Self {
        Self::LoadError { path, source }
    }

    /// Create an InvalidFormat error
    pub fn invalid_format(format: impl Into<String>) -> Self {
        Self::InvalidFormat { format: format.into() }
    }

    /// Create a CacheError
    pub fn cache_error(message: impl Into<String>) -> Self {
        Self::CacheError { message: message.into() }
    }

    /// Create a StrategyError
    pub fn strategy_error(strategy: impl Into<String>, message: impl Into<String>) -> Self {
        Self::StrategyError { 
            strategy: strategy.into(), 
            message: message.into() 
        }
    }

    /// Create a ConfigError
    pub fn config_error(message: impl Into<String>) -> Self {
        Self::ConfigError { message: message.into() }
    }
}