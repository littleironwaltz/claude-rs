use claude_rs::{Claude, ClaudeModel};
use claude_rs::domains::DomainClient;
use std::sync::Arc;
use tokio::task;

// A simple custom domain client to demonstrate domain registry
#[allow(dead_code)]
struct CustomDomainClient {
    name: String,
    claude: Arc<Claude>,
}

impl CustomDomainClient {
    fn new(name: String, claude: Arc<Claude>) -> Self {
        Self { name, claude }
    }
    
    #[allow(dead_code)]
    async fn perform_operation(&self, input: &str) -> String {
        format!("Domain '{}' processed: {}", self.name, input)
    }
}

impl DomainClient for CustomDomainClient {
    fn domain_name(&self) -> &str {
        &self.name
    }
}

// Create and register multiple domain clients concurrently
async fn register_domains(client: Arc<Claude>, count: usize) {
    let mut handles = Vec::new();
    
    // Spawn tasks to register domains concurrently
    for i in 0..count {
        let client_clone = client.clone();
        let handle = task::spawn(async move {
            let domain_name = format!("custom_domain_{}", i);
            let custom_client = CustomDomainClient::new(domain_name.clone(), client_clone.clone());
            client_clone.domains().register(&domain_name, custom_client);
            
            // Small delay to simulate real-world concurrent operations
            tokio::time::sleep(tokio::time::Duration::from_millis(1)).await;
            
            domain_name
        });
        handles.push(handle);
    }
    
    // Wait for all registrations to complete
    for handle in handles {
        let domain_name = handle.await.unwrap();
        println!("Registered domain: {}", domain_name);
    }
}

// Access domains concurrently
async fn access_domains(client: Arc<Claude>, domains: Vec<String>) {
    let mut handles = Vec::new();
    
    // Spawn tasks to access domains concurrently
    for domain_name in domains {
        let client_clone = client.clone();
        let handle = task::spawn(async move {
            let domain_client = client_clone.domains().get(&domain_name);
            match domain_client {
                Some(client) => {
                    // Small delay to simulate processing time
                    tokio::time::sleep(tokio::time::Duration::from_millis(1)).await;
                    
                    // Extract the domain name from the client for our result
                    client.domain_name().to_string()
                },
                None => "Not found".to_string(),
            }
        });
        handles.push(handle);
    }
    
    // Wait for all access operations to complete
    for handle in handles {
        let result = handle.await.unwrap();
        println!("Retrieved domain: {}", result);
    }
}

// Benchmark cached vs. regular domain access
async fn benchmark_domain_access(client: Arc<Claude>, iterations: usize) {
    use std::time::Instant;
    
    // 1. Benchmark cached domain access (OnceLock-based)
    let start = Instant::now();
    for _ in 0..iterations {
        let _sentiment = client.sentiment();
        let _entity = client.entity();
        let _content = client.content();
        let _code = client.code();
    }
    let cached_duration = start.elapsed();
    
    // 2. Benchmark custom domain access (DashMap-based)
    // First, register some custom domains if they don't exist
    for i in 0..4 {
        let domain_name = format!("bench_domain_{}", i);
        if client.domains().get(&domain_name).is_none() {
            let custom_client = CustomDomainClient::new(domain_name.clone(), client.clone());
            client.domains().register(&domain_name, custom_client);
        }
    }
    
    // Now benchmark access
    let start = Instant::now();
    for _ in 0..iterations {
        let _domain0 = client.domains().get("bench_domain_0");
        let _domain1 = client.domains().get("bench_domain_1");
        let _domain2 = client.domains().get("bench_domain_2");
        let _domain3 = client.domains().get("bench_domain_3");
    }
    let registry_duration = start.elapsed();
    
    // Print results
    println!("\nBenchmark Results ({} iterations):", iterations);
    println!("Cached domain access: {:?} ({:.2} ns/op)", cached_duration, 
             cached_duration.as_nanos() as f64 / iterations as f64);
    println!("Registry domain access: {:?} ({:.2} ns/op)", registry_duration, 
             registry_duration.as_nanos() as f64 / iterations as f64);
    println!("Difference: {:.2}x faster with caching", 
             registry_duration.as_nanos() as f64 / cached_duration.as_nanos() as f64);
}

#[tokio::main]
async fn main() {
    println!("Claude-rs Concurrent Domain Registry Example");
    println!("===========================================\n");
    
    // Create a Claude client
    let client = Arc::new(Claude::new("test-key").with_model(ClaudeModel::Sonnet));
    
    // 1. Register multiple domain clients concurrently
    println!("1. Registering 10 custom domains concurrently...");
    register_domains(client.clone(), 10).await;
    
    // 2. Access both built-in and custom domains
    println!("\n2. Accessing domains concurrently...");
    
    // First, demonstrate that built-in domains work
    println!("\nBuilt-in domains:");
    let sentiment = client.sentiment();
    println!("Sentiment domain: {}", sentiment.domain_name());
    
    let entity = client.entity();
    println!("Entity domain: {}", entity.domain_name());
    
    // Now access custom domains concurrently
    println!("\nCustom domains:");
    let domains = (0..10).map(|i| format!("custom_domain_{}", i)).collect();
    access_domains(client.clone(), domains).await;
    
    // 3. Demonstrate performance differences
    println!("\n3. Performance comparison:");
    benchmark_domain_access(client.clone(), 100_000).await;
    
    // 4. List some known domains that should exist
    println!("\n4. Domain registry contents:");
    // Note: We don't need to look up built-in domains in the registry
    // since they're accessed via OnceLock
    let custom_domains: Vec<String> = (0..14).map(|i| format!("custom_domain_{}", i)).collect();
    let bench_domains: Vec<String> = (0..4).map(|i| format!("bench_domain_{}", i)).collect();
    
    // Verify domains exist
    println!("Accessing built-in domains:");
    println!("  ✅ sentiment domain: {}", client.sentiment().domain_name());
    println!("  ✅ entity domain: {}", client.entity().domain_name());
    println!("  ✅ content domain: {}", client.content().domain_name());
    println!("  ✅ code domain: {}", client.code().domain_name());
    println!("\nNote: Built-in domains are accessed via OnceLock caching, not the domain registry map");
    
    println!("\nVerifying custom domains:");
    let mut found_count = 0;
    for domain in &custom_domains {
        if client.domains().get(domain).is_some() {
            found_count += 1;
        }
    }
    println!("Found {}/{} custom domains", found_count, custom_domains.len());
    
    println!("\nVerifying benchmark domains:");
    let mut found_count = 0;
    for domain in &bench_domains {
        if client.domains().get(domain).is_some() {
            found_count += 1;
        }
    }
    println!("Found {}/{} benchmark domains", found_count, bench_domains.len());
    
    // Calculate total domains
    let total_domains = 4 + found_count; // 4 built-in domains + custom domains
    println!("\nTotal verified domains: {}", total_domains);
}