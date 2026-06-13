#[cfg(test)]
mod verification_tests {
    use crate::constants::{network, timing};
    use crate::core::service::ServiceManager;
    use std::time::Duration;

    #[test]
    fn test_service_wait_interval_non_zero() {
        #[cfg(target_os = "windows")]
        {
            assert!(timing::SERVICE_WAIT_INTERVAL > Duration::from_millis(0));
            assert_eq!(timing::SERVICE_WAIT_INTERVAL, Duration::from_millis(200));
        }
    }

    #[test]
    fn test_retry_delay_non_zero() {
        let config = ServiceManager::config();
        assert!(config.retry_delay > Duration::from_millis(0));
        assert_eq!(config.retry_delay, Duration::from_millis(250));
    }

    #[test]
    fn test_ports_match_agreements() {
        assert_eq!(network::ports::DEFAULT_MIXED, 10801);
        assert_eq!(network::DEFAULT_EXTERNAL_CONTROLLER, "127.0.0.1:9098");
        
        // Singleton server ports (not verge-dev / verge-dev)
        // Since we can't test both cfg branches at once in a single run,
        // we check the active one, or verify both if we know the compilation flags.
        #[cfg(not(feature = "verge-dev"))]
        assert_eq!(network::ports::SINGLETON_SERVER, 33335);
        #[cfg(feature = "verge-dev")]
        assert_eq!(network::ports::SINGLETON_SERVER, 33336);
    }
}
