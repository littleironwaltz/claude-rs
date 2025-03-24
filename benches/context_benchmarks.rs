use criterion::{criterion_group, criterion_main, Criterion, BenchmarkId};
use claude_rs::{AdaptiveContextManager, ImportanceScorer, SimpleImportanceScorer, ContextManager}; 
use claude_rs::{Content, Message, Role};
use tokio::runtime::Runtime;
use async_trait::async_trait;

pub fn context_manager_benchmark(c: &mut Criterion) {
    let runtime = Runtime::new().unwrap();
    
    let mut group = c.benchmark_group("context_manager");
    
    // Benchmark token estimation using a simplified approach
    for text_size in [100, 1000, 10000] {
        let text = "a".repeat(text_size);
        group.bench_with_input(BenchmarkId::new("token_estimation", text_size), &text, |b, text| {
            // Using a simple approximation of token counting
            b.iter(|| {
                let tokens = text.len() as u32 / 4; // simple approximation (4 chars per token)
                criterion::black_box(tokens)
            });
        });
    }
    
    // Custom scorer for benchmarking
    #[allow(dead_code)]
    struct BenchScorer;
    
    #[async_trait]
    impl ImportanceScorer for BenchScorer {
        async fn score_importance(&self, message: &Message) -> f32 {
            let mut score = 0.5;
            
            for content in &message.content {
                if let Content::Text { text } = content {
                    score += text.len() as f32 / 10000.0;
                }
            }
            
            score.min(1.0)
        }
    }
    
    // Benchmark message processing with different context sizes
    for size in [5, 20, 100] {
        group.bench_with_input(BenchmarkId::new("process_messages", size), &size, |b, &size| {
            let context_manager = AdaptiveContextManager::new(4000, SimpleImportanceScorer);
            
            // Create test messages
            let messages: Vec<Message> = (0..size)
                .map(|i| Message {
                    role: if i % 2 == 0 { Role::User } else { Role::Assistant },
                    content: vec![Content::Text {
                        text: format!("Message content {}", i).repeat(10),
                    }],
                })
                .collect();
            
            b.iter(|| {
                runtime.block_on(async {
                    // Process messages using the ContextManager trait
                    let result = ContextManager::process_messages(&context_manager, messages.clone()).await;
                    criterion::black_box(result)
                })
            });
        });
    }
    
    group.finish();
}

criterion_group!(
    benches,
    context_manager_benchmark
);
criterion_main!(benches);