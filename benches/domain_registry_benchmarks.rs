use criterion::{criterion_group, criterion_main, Criterion};
use claude_rs::{Claude, ClaudeModel};
use claude_rs::domains::DomainClient;
use std::sync::Arc;
use std::io::Write;
use std::fs::File;

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

fn bench_domain_registry_access(c: &mut Criterion) {
    // Create a log file
    let mut file = File::create("domain_benchmark.log").unwrap();
    
    writeln!(file, "Starting domain registry benchmarks...").unwrap();
    
    let mut group = c.benchmark_group("domain_registry_access");
    
    // Create a client
    let client = Arc::new(Claude::new("test-key").with_model(ClaudeModel::Sonnet));
    
    // Register some custom clients
    for i in 0..100 {
        let name = format!("test_domain_{}", i);
        let custom_client = TestDomainClient::new(&name);
        client.domains().register(&name, custom_client);
    }
    
    writeln!(file, "Registered 100 test domains").unwrap();
    
    // Benchmark cached access
    writeln!(file, "Running cached domain access benchmark").unwrap();
    
    group.bench_function("cached_domain_access", |b| {
        b.iter(|| {
            let _sentiment = client.sentiment();
            let _entity = client.entity();
            let _content = client.content();
            let _code = client.code();
        });
    });
    
    writeln!(file, "Cached domain access benchmark complete").unwrap();
    
    // Benchmark registry lookup
    writeln!(file, "Running registry lookup benchmark").unwrap();
    group.bench_function("registry_lookup", |b| {
        b.iter(|| {
            for i in 0..10 {
                let name = format!("test_domain_{}", i);
                let _retrieved = client.domains().get(&name);
            }
        });
    });
    writeln!(file, "Registry lookup benchmark complete").unwrap();
    
    // Benchmark domain registration
    writeln!(file, "Running domain registration benchmark").unwrap();
    group.bench_function("domain_registration", |b| {
        let mut counter = 100;
        b.iter(|| {
            let name = format!("bench_domain_{}", counter);
            counter += 1;
            // Create client directly without Arc wrapper
            let custom_client = TestDomainClient::new(&name);
            client.domains().register(&name, custom_client);
        });
    });
    writeln!(file, "Domain registration benchmark complete").unwrap();
    
    group.finish();
}

fn bench_domain_client_creation(c: &mut Criterion) {
    // Append to the log file
    let mut file = std::fs::OpenOptions::new()
        .append(true)
        .open("domain_benchmark.log")
        .unwrap();
    
    writeln!(file, "Starting domain client creation benchmarks...").unwrap();
    
    let mut group = c.benchmark_group("domain_client_creation");
    
    // Create a client
    let client = Arc::new(Claude::new("test-key").with_model(ClaudeModel::Sonnet));
    
    // Benchmark domain client creation through the factory methods
    writeln!(file, "Running client factory methods benchmark").unwrap();
    group.bench_function("client_factory_methods", |b| {
        b.iter(|| {
            let _sentiment = Claude::new("test-key").sentiment();
            let _entity = Claude::new("test-key").entity();
            let _content = Claude::new("test-key").content();
            let _code = Claude::new("test-key").code();
        });
    });
    writeln!(file, "Client factory methods benchmark complete").unwrap();
    
    // Benchmark domain client creation through the cached accessors
    writeln!(file, "Running cached accessors benchmark").unwrap();
    group.bench_function("cached_accessors", |b| {
        b.iter(|| {
            let _sentiment = client.sentiment();
            let _entity = client.entity();
            let _content = client.content();
            let _code = client.code();
        });
    });
    writeln!(file, "Cached accessors benchmark complete").unwrap();
    
    group.finish();
}

criterion_group!(benches, bench_domain_registry_access, bench_domain_client_creation);
criterion_main!(benches);