use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use tracing::{debug, info, warn, error};

use crate::icon::types::{IconContext, IconResult, IconError};
use crate::icon::traits::{IconDetectionStrategy, StrategyProvider};

/// Manages and executes icon detection strategies in priority order
/// 
/// The IconResolver maintains a collection of strategies and executes them
/// in priority order until one successfully finds an icon. It provides
/// strategy registration, priority ordering, and fallback logic.
pub struct IconResolver {
    /// Registered strategies, sorted by priority (highest first)
    strategies: Arc<RwLock<Vec<Box<dyn IconDetectionStrategy>>>>,
    /// Strategy execution statistics for monitoring
    stats: Arc<RwLock<HashMap<String, StrategyStats>>>,
}

impl std::fmt::Debug for IconResolver {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let strategy_count = self.strategy_count();
        f.debug_struct("IconResolver")
            .field("strategy_count", &strategy_count)
            .finish()
    }
}

/// Statistics for strategy execution monitoring
#[derive(Debug, Clone, Default)]
struct StrategyStats {
    /// Number of times this strategy was attempted
    attempts: u64,
    /// Number of successful icon detections
    successes: u64,
    /// Total execution time in microseconds
    total_execution_time_us: u64,
}

impl IconResolver {
    /// Create a new IconResolver with no strategies
    pub fn new() -> Self {
        Self {
            strategies: Arc::new(RwLock::new(Vec::new())),
            stats: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Register a single strategy
    /// 
    /// The strategy will be inserted in the correct position based on its priority.
    /// Higher priority strategies are executed first.
    pub fn register_strategy(&mut self, mut strategy: Box<dyn IconDetectionStrategy>) -> Result<(), IconError> {
        let strategy_name = strategy.name().to_string();
        
        // Initialize the strategy
        if let Err(e) = strategy.initialize() {
            error!("Failed to initialize strategy '{}': {}", strategy_name, e);
            return Err(IconError::strategy_error(strategy_name, format!("Initialization failed: {}", e)));
        }

        // Check if strategy is available
        if !strategy.is_available() {
            warn!("Strategy '{}' is not available, skipping registration", strategy_name);
            return Ok(());
        }

        let priority = strategy.priority();
        
        // Insert strategy in priority order (highest priority first)
        let mut strategies = self.strategies.write().map_err(|e| {
            IconError::strategy_error(&strategy_name, format!("Failed to acquire write lock: {}", e))
        })?;

        let insert_pos = strategies
            .iter()
            .position(|s| s.priority() < priority)
            .unwrap_or(strategies.len());

        strategies.insert(insert_pos, strategy);

        // Initialize stats for this strategy
        let mut stats = self.stats.write().map_err(|e| {
            IconError::strategy_error(&strategy_name, format!("Failed to acquire stats write lock: {}", e))
        })?;
        stats.insert(strategy_name.clone(), StrategyStats::default());

        info!("Registered strategy '{}' with priority {}", strategy_name, priority);
        Ok(())
    }

    /// Register multiple strategies from a provider
    pub fn register_provider(&mut self, provider: Box<dyn StrategyProvider>) -> Result<(), IconError> {
        let provider_name = provider.provider_name();
        debug!("Registering strategies from provider '{}'", provider_name);

        let strategies = provider.get_strategies();
        let mut registered_count = 0;
        let mut errors = Vec::new();

        for strategy in strategies {
            match self.register_strategy(strategy) {
                Ok(()) => registered_count += 1,
                Err(e) => errors.push(e),
            }
        }

        if !errors.is_empty() {
            warn!("Provider '{}' had {} registration errors", provider_name, errors.len());
            for error in &errors {
                warn!("  - {}", error);
            }
        }

        info!("Provider '{}' registered {} strategies", provider_name, registered_count);
        Ok(())
    }

    /// Remove a strategy by name
    pub fn remove_strategy(&mut self, name: &str) -> Result<bool, IconError> {
        let mut strategies = self.strategies.write().map_err(|e| {
            IconError::strategy_error(name, format!("Failed to acquire write lock: {}", e))
        })?;

        if let Some(pos) = strategies.iter().position(|s| s.name() == name) {
            let mut removed_strategy = strategies.remove(pos);
            removed_strategy.cleanup();
            
            // Remove stats
            let mut stats = self.stats.write().map_err(|e| {
                IconError::strategy_error(name, format!("Failed to acquire stats write lock: {}", e))
            })?;
            stats.remove(name);

            info!("Removed strategy '{}'", name);
            Ok(true)
        } else {
            debug!("Strategy '{}' not found for removal", name);
            Ok(false)
        }
    }

    /// Get list of registered strategy names in priority order
    pub fn list_strategies(&self) -> Result<Vec<String>, IconError> {
        let strategies = self.strategies.read().map_err(|e| {
            IconError::strategy_error("resolver", format!("Failed to acquire read lock: {}", e))
        })?;

        Ok(strategies.iter().map(|s| s.name().to_string()).collect())
    }

    /// Get strategy execution statistics
    pub fn get_stats(&self) -> Result<HashMap<String, (u64, u64, f64)>, IconError> {
        let stats = self.stats.read().map_err(|e| {
            IconError::strategy_error("resolver", format!("Failed to acquire stats read lock: {}", e))
        })?;

        Ok(stats.iter().map(|(name, stat)| {
            let avg_time_us = if stat.attempts > 0 {
                stat.total_execution_time_us as f64 / stat.attempts as f64
            } else {
                0.0
            };
            (name.clone(), (stat.attempts, stat.successes, avg_time_us))
        }).collect())
    }

    /// Clear all strategies and statistics
    pub fn clear(&mut self) -> Result<(), IconError> {
        let mut strategies = self.strategies.write().map_err(|e| {
            IconError::strategy_error("resolver", format!("Failed to acquire write lock: {}", e))
        })?;

        // Cleanup all strategies
        for strategy in strategies.iter_mut() {
            strategy.cleanup();
        }
        strategies.clear();

        // Clear stats
        let mut stats = self.stats.write().map_err(|e| {
            IconError::strategy_error("resolver", format!("Failed to acquire stats write lock: {}", e))
        })?;
        stats.clear();

        info!("Cleared all strategies and statistics");
        Ok(())
    }

    /// Resolve an icon using registered strategies
    /// 
    /// Executes strategies in priority order until one returns a successful result.
    /// Returns the first successful result, or None if all strategies fail.
    pub fn resolve(&self, context: &IconContext) -> Option<IconResult> {
        let strategies = match self.strategies.read() {
            Ok(s) => s,
            Err(e) => {
                error!("Failed to acquire read lock for strategies: {}", e);
                return None;
            }
        };

        if strategies.is_empty() {
            warn!("No strategies registered for icon resolution");
            return None;
        }

        debug!("Resolving icon for class '{}' using {} strategies", 
               context.class, strategies.len());

        for strategy in strategies.iter() {
            let strategy_name = strategy.name();
            
            // Check if strategy is still available
            if !strategy.is_available() {
                debug!("Strategy '{}' is not available, skipping", strategy_name);
                continue;
            }

            debug!("Trying strategy '{}' for class '{}'", strategy_name, context.class);
            
            let start_time = std::time::Instant::now();
            let result = strategy.detect_icon(context);
            let execution_time = start_time.elapsed();

            // Update statistics
            if let Ok(mut stats) = self.stats.write() {
                let strategy_stats = stats.entry(strategy_name.to_string()).or_default();
                strategy_stats.attempts += 1;
                strategy_stats.total_execution_time_us += execution_time.as_micros() as u64;

                if result.is_some() {
                    strategy_stats.successes += 1;
                }
            }

            match result {
                Some(icon_result) => {
                    info!("Strategy '{}' found icon for class '{}': {:?}", 
                          strategy_name, context.class, icon_result.path);
                    debug!("Strategy '{}' execution time: {:?}", strategy_name, execution_time);
                    return Some(icon_result);
                }
                None => {
                    debug!("Strategy '{}' failed to find icon for class '{}'", 
                           strategy_name, context.class);
                }
            }
        }

        warn!("All {} strategies failed to find icon for class '{}'", 
              strategies.len(), context.class);
        None
    }

    /// Get the number of registered strategies
    pub fn strategy_count(&self) -> usize {
        self.strategies.read().map(|s| s.len()).unwrap_or(0)
    }

    /// Check if a strategy with the given name is registered
    pub fn has_strategy(&self, name: &str) -> bool {
        self.strategies.read()
            .map(|s| s.iter().any(|strategy| strategy.name() == name))
            .unwrap_or(false)
    }
}

impl Default for IconResolver {
    fn default() -> Self {
        Self::new()
    }
}

// Implement Drop to ensure cleanup
impl Drop for IconResolver {
    fn drop(&mut self) {
        if let Err(e) = self.clear() {
            error!("Error during IconResolver cleanup: {}", e);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use crate::icon::types::{IconMetadata, IconFormat};

    // Mock strategy for testing
    struct MockStrategy {
        name: &'static str,
        priority: u8,
        should_succeed: bool,
        available: bool,
    }

    impl MockStrategy {
        fn new(name: &'static str, priority: u8, should_succeed: bool) -> Self {
            Self {
                name,
                priority,
                should_succeed,
                available: true,
            }
        }

        fn unavailable(name: &'static str, priority: u8) -> Self {
            Self {
                name,
                priority,
                should_succeed: false,
                available: false,
            }
        }
    }

    impl IconDetectionStrategy for MockStrategy {
        fn detect_icon(&self, context: &IconContext) -> Option<IconResult> {
            if self.should_succeed {
                Some(IconResult::new(
                    PathBuf::from(format!("/mock/{}.png", context.class)),
                    self.name.to_string(),
                    1.0,
                    IconMetadata::new(IconFormat::Png),
                ))
            } else {
                None
            }
        }

        fn priority(&self) -> u8 {
            self.priority
        }

        fn name(&self) -> &'static str {
            self.name
        }

        fn is_available(&self) -> bool {
            self.available
        }
    }

    #[test]
    fn test_resolver_creation() {
        let resolver = IconResolver::new();
        assert_eq!(resolver.strategy_count(), 0);
    }

    #[test]
    fn test_strategy_registration() {
        let mut resolver = IconResolver::new();
        
        let strategy = Box::new(MockStrategy::new("test", 50, true));
        resolver.register_strategy(strategy).unwrap();
        
        assert_eq!(resolver.strategy_count(), 1);
        assert!(resolver.has_strategy("test"));
    }

    #[test]
    fn test_strategy_priority_ordering() {
        let mut resolver = IconResolver::new();
        
        // Register strategies in random order
        resolver.register_strategy(Box::new(MockStrategy::new("low", 10, false))).unwrap();
        resolver.register_strategy(Box::new(MockStrategy::new("high", 90, true))).unwrap();
        resolver.register_strategy(Box::new(MockStrategy::new("medium", 50, false))).unwrap();
        
        let strategies = resolver.list_strategies().unwrap();
        assert_eq!(strategies, vec!["high", "medium", "low"]);
    }

    #[test]
    fn test_strategy_execution_order() {
        let mut resolver = IconResolver::new();
        
        // Register high priority failing strategy and low priority succeeding strategy
        resolver.register_strategy(Box::new(MockStrategy::new("high_fail", 90, false))).unwrap();
        resolver.register_strategy(Box::new(MockStrategy::new("low_success", 10, true))).unwrap();
        
        let context = IconContext::new("test-class".to_string());
        let result = resolver.resolve(&context);
        
        assert!(result.is_some());
        let result = result.unwrap();
        assert_eq!(result.strategy_used, "low_success");
    }

    #[test]
    fn test_first_success_wins() {
        let mut resolver = IconResolver::new();
        
        // Register multiple succeeding strategies
        resolver.register_strategy(Box::new(MockStrategy::new("first", 90, true))).unwrap();
        resolver.register_strategy(Box::new(MockStrategy::new("second", 80, true))).unwrap();
        
        let context = IconContext::new("test-class".to_string());
        let result = resolver.resolve(&context);
        
        assert!(result.is_some());
        let result = result.unwrap();
        assert_eq!(result.strategy_used, "first");
    }

    #[test]
    fn test_unavailable_strategy_skipped() {
        let mut resolver = IconResolver::new();
        
        // Register unavailable high priority and available low priority
        resolver.register_strategy(Box::new(MockStrategy::unavailable("unavailable", 90))).unwrap();
        resolver.register_strategy(Box::new(MockStrategy::new("available", 10, true))).unwrap();
        
        let context = IconContext::new("test-class".to_string());
        let result = resolver.resolve(&context);
        
        assert!(result.is_some());
        let result = result.unwrap();
        assert_eq!(result.strategy_used, "available");
    }

    #[test]
    fn test_no_strategies_returns_none() {
        let resolver = IconResolver::new();
        let context = IconContext::new("test-class".to_string());
        let result = resolver.resolve(&context);
        
        assert!(result.is_none());
    }

    #[test]
    fn test_all_strategies_fail() {
        let mut resolver = IconResolver::new();
        
        resolver.register_strategy(Box::new(MockStrategy::new("fail1", 90, false))).unwrap();
        resolver.register_strategy(Box::new(MockStrategy::new("fail2", 80, false))).unwrap();
        
        let context = IconContext::new("test-class".to_string());
        let result = resolver.resolve(&context);
        
        assert!(result.is_none());
    }

    #[test]
    fn test_strategy_removal() {
        let mut resolver = IconResolver::new();
        
        resolver.register_strategy(Box::new(MockStrategy::new("test", 50, true))).unwrap();
        assert!(resolver.has_strategy("test"));
        
        let removed = resolver.remove_strategy("test").unwrap();
        assert!(removed);
        assert!(!resolver.has_strategy("test"));
        assert_eq!(resolver.strategy_count(), 0);
    }

    #[test]
    fn test_remove_nonexistent_strategy() {
        let mut resolver = IconResolver::new();
        let removed = resolver.remove_strategy("nonexistent").unwrap();
        assert!(!removed);
    }

    #[test]
    fn test_clear_strategies() {
        let mut resolver = IconResolver::new();
        
        resolver.register_strategy(Box::new(MockStrategy::new("test1", 50, true))).unwrap();
        resolver.register_strategy(Box::new(MockStrategy::new("test2", 40, true))).unwrap();
        
        assert_eq!(resolver.strategy_count(), 2);
        
        resolver.clear().unwrap();
        assert_eq!(resolver.strategy_count(), 0);
    }

    #[test]
    fn test_statistics_tracking() {
        let mut resolver = IconResolver::new();
        
        resolver.register_strategy(Box::new(MockStrategy::new("success", 90, true))).unwrap();
        resolver.register_strategy(Box::new(MockStrategy::new("fail", 80, false))).unwrap();
        
        let context = IconContext::new("test-class".to_string());
        
        // Execute multiple times
        for _ in 0..3 {
            resolver.resolve(&context);
        }
        
        let stats = resolver.get_stats().unwrap();
        
        // Success strategy should have been called once per resolution (first success wins)
        assert_eq!(stats.get("success").unwrap().0, 3); // attempts
        assert_eq!(stats.get("success").unwrap().1, 3); // successes
        
        // Fail strategy should never be called (success strategy wins first)
        assert_eq!(stats.get("fail").unwrap().0, 0); // attempts
        assert_eq!(stats.get("fail").unwrap().1, 0); // successes
    }

    // Mock provider for testing
    struct MockProvider {
        strategies: Vec<Box<dyn IconDetectionStrategy>>,
    }

    impl MockProvider {
        fn new() -> Self {
            Self {
                strategies: vec![
                    Box::new(MockStrategy::new("provider_strategy1", 60, true)),
                    Box::new(MockStrategy::new("provider_strategy2", 40, false)),
                ],
            }
        }
    }

    impl StrategyProvider for MockProvider {
        fn get_strategies(&self) -> Vec<Box<dyn IconDetectionStrategy>> {
            // Note: In a real implementation, we'd need to clone or recreate strategies
            // For this test, we'll create new instances
            vec![
                Box::new(MockStrategy::new("provider_strategy1", 60, true)),
                Box::new(MockStrategy::new("provider_strategy2", 40, false)),
            ]
        }

        fn provider_name(&self) -> &'static str {
            "MockProvider"
        }
    }

    #[test]
    fn test_provider_registration() {
        let mut resolver = IconResolver::new();
        let provider = Box::new(MockProvider::new());
        
        resolver.register_provider(provider).unwrap();
        
        assert_eq!(resolver.strategy_count(), 2);
        assert!(resolver.has_strategy("provider_strategy1"));
        assert!(resolver.has_strategy("provider_strategy2"));
    }
}