#[cfg(test)]
mod tests {
    use claude_rs::{Claude, ClaudeModel};
    use claude_rs::domains::DomainClient;
    use std::sync::Arc;
    use std::time::{Instant, Duration};
    use std::thread;

    // Helper to measure execution time
    fn measure_time<F, T>(f: F) -> (T, Duration)
    where
        F: FnOnce() -> T,
    {
        let start = Instant::now();
        let result = f();
        let duration = start.elapsed();
        (result, duration)
    }

    #[test]
    fn test_domain_client_caching() {
        let client = Arc::new(Claude::new("test-key").with_model(ClaudeModel::Sonnet));
        
        // First access creates the client
        let (_, first_duration) = measure_time(|| {
            client.sentiment()
        });
        
        // Subsequent accesses should be much faster due to caching
        let (_, second_duration) = measure_time(|| {
            client.sentiment()
        });
        
        println!("First access: {:?}", first_duration);
        println!("Second access: {:?}", second_duration);
        
        // The second access should be significantly faster
        assert!(second_duration < first_duration / 2);
    }
    
    #[test]
    fn test_concurrent_domain_registry_access() {
        let client = Arc::new(Claude::new("test-key").with_model(ClaudeModel::Sonnet));
        
        // Prepare for multi-threaded test
        let mut handles = vec![];
        
        // Spawn 10 threads that each access the registry 100 times
        for thread_id in 0..10 {
            let client_clone = client.clone();
            let handle = thread::spawn(move || {
                for i in 0..100 {
                    let _sentiment = client_clone.sentiment();
                    let _entity = client_clone.entity();
                    let _content = client_clone.content();
                    let _code = client_clone.code();
                    
                    // Register and get a custom domain client
                    let name = format!("test_domain_{}_{}", thread_id, i);
                    let custom_client = TestDomainClient::new(&name);
                    
                    // Register with the domain registry
                    client_clone.domains().register(&name, custom_client);
                    
                    // Get from the domain registry
                    let retrieved = client_clone.domains().get(&name);
                    assert!(retrieved.is_some());
                }
            });
            handles.push(handle);
        }
        
        // Wait for all threads to complete
        for handle in handles {
            handle.join().unwrap();
        }
        
        // If we got here without panics or deadlocks, the test passed!
    }
    
    #[test]
    fn test_domain_cache_and_registry_integration() {
        let client = Arc::new(Claude::new("test-key").with_model(ClaudeModel::Sonnet));
        
        // Access the cached domains
        let sentiment1 = client.sentiment();
        let sentiment2 = client.sentiment();
        
        // They should be the same Arc instance
        assert!(Arc::ptr_eq(&sentiment1, &sentiment2));
        
        // Register a custom client
        let custom_client = TestDomainClient::new("custom");
        client.domains().register("custom", custom_client);
        
        // Retrieve it
        let retrieved = client.domains().get("custom").unwrap();
        
        // Check domain name
        assert_eq!(retrieved.domain_name(), "custom");
        
        // Access all domains many times in a row to stress test
        for _ in 0..1000 {
            let _s = client.sentiment();
            let _e = client.entity();
            let _code = client.code();
            let _content = client.content();
            let _custom = client.domains().get("custom");
        }
    }
    
    // Simple test domain client
    struct TestDomainClient {
        name: String,
    }
    
    impl TestDomainClient {
        fn new(name: &str) -> Self {
            Self {
                name: name.to_string(),
            }
        }
    }
    
    impl DomainClient for TestDomainClient {
        fn domain_name(&self) -> &str {
            &self.name
        }
    }
}