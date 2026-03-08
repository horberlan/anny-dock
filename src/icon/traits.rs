use crate::icon::types::{IconContext, IconResult};

/// Trait for icon detection strategies
/// 
/// Each strategy implements a different method for finding icons based on
/// the provided context information. Strategies are executed in priority order
/// until one returns a successful result.
pub trait IconDetectionStrategy: Send + Sync {
    /// Attempt to detect an icon for the given context
    /// 
    /// Returns Some(IconResult) if an icon is found, None otherwise.
    /// Should not panic - any errors should be logged and None returned.
    fn detect_icon(&self, context: &IconContext) -> Option<IconResult>;

    /// Priority of this strategy (higher values = higher priority)
    /// 
    /// Strategies with higher priority values are executed first.
    /// Typical priority ranges:
    /// - 100+: Custom/user-defined paths (highest priority)
    /// - 50-99: Application-specific strategies
    /// - 10-49: General detection strategies
    /// - 1-9: Fallback strategies (lowest priority)
    fn priority(&self) -> u8;

    /// Human-readable name of this strategy for logging and debugging
    fn name(&self) -> &'static str;

    /// Optional: Check if this strategy is available/enabled
    /// 
    /// Default implementation returns true. Strategies can override this
    /// to check for required dependencies, configuration, etc.
    fn is_available(&self) -> bool {
        true
    }

    /// Optional: Initialize the strategy
    /// 
    /// Called once when the strategy is registered. Can be used for
    /// one-time setup, validation, etc. Default implementation does nothing.
    fn initialize(&mut self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        Ok(())
    }

    /// Optional: Cleanup when strategy is no longer needed
    /// 
    /// Called when the strategy is being removed or the system is shutting down.
    /// Default implementation does nothing.
    fn cleanup(&mut self) {}
}

/// Trait for components that can provide icon detection strategies
/// 
/// This allows for modular registration of strategies from different sources
pub trait StrategyProvider {
    /// Get all strategies provided by this provider
    fn get_strategies(&self) -> Vec<Box<dyn IconDetectionStrategy>>;
    
    /// Get the name of this provider for logging
    fn provider_name(&self) -> &'static str;
}