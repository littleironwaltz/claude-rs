use criterion::{criterion_group, criterion_main, Criterion, BenchmarkId};
use claude_rs::client::Claude;
use claude_rs::types::ClaudeModel;
use std::sync::Arc;

pub fn client_construction_benchmark(c: &mut Criterion) {
    let mut group = c.benchmark_group("client_construction");
    
    group.bench_function("new_client", |b| {
        b.iter(|| {
            let _client = Claude::new("fake-api-key");
        });
    });
    
    group.bench_function("with_model", |b| {
        b.iter(|| {
            let _client = Claude::new("fake-api-key")
                .with_model(ClaudeModel::Sonnet);
        });
    });
    
    group.finish();
}

pub fn message_builder_benchmark(c: &mut Criterion) {
    let client = Claude::new("fake-api-key");
    
    let mut group = c.benchmark_group("message_builder");
    
    group.bench_function("simple_message", |b| {
        b.iter(|| {
            // Ignore the actual result for benchmarking
            let _ = client.message()
                .user_message("Hello, Claude!")
                .map(|builder| builder.model(ClaudeModel::Haiku));
        });
    });
    
    // Let's simplify to focus on the basic benchmarking case
    group.bench_function("complex_message", |b| {
        b.iter(|| {
            // Chain Result methods for proper error handling in benchmarks
            let _ = client.message()
                .user_message("Hello, Claude!")
                .map(|builder| builder.model(ClaudeModel::Opus));
        });
    });
    
    group.finish();
}

pub fn domain_client_benchmark(c: &mut Criterion) {
    let client = Arc::new(Claude::new("fake-api-key"));
    
    let mut group = c.benchmark_group("domain_client");
    
    group.bench_function("get_sentiment_client", |b| {
        b.iter(|| {
            let _sentiment = client.sentiment();
        });
    });
    
    group.bench_function("get_code_client", |b| {
        b.iter(|| {
            let _code = client.code();
        });
    });
    
    for i in [1, 10, 100] {
        group.bench_with_input(BenchmarkId::new("repeated_domain_access", i), &i, |b, &i| {
            b.iter(|| {
                for _ in 0..i {
                    let _sentiment = client.sentiment();
                    let _entity = client.entity();
                    let _content = client.content();
                    let _code = client.code();
                }
            });
        });
    }
    
    group.finish();
}

#[cfg(feature = "reactive")]
pub fn streaming_benchmark(c: &mut Criterion) {
    use claude_rs::client::MockApiHandler;
    use claude_rs::types::{Content, DeltaEvent, DeltaMessage, Role, Delta, Usage};
    use futures::StreamExt;
    use std::pin::Pin;
    use std::future::Future;
    
    // Create a runtime for async benchmarks
    let rt = tokio::runtime::Runtime::new().unwrap();
    
    // Create mock handler for streaming
    struct StreamingMock;
    
    impl StreamingMock {
        fn create_delta_events() -> Vec<DeltaEvent> {
            vec![
                // Message start event
                DeltaEvent {
                    event_type: "message_start".to_string(),
                    message: Some(DeltaMessage {
                        id: "msg_sample123".to_string(),
                        model: "claude-3-sonnet-20240229".to_string(),
                        content: None,
                        stop_reason: None,
                        stop_sequence: None,
                        role: Some(Role::Assistant),
                        type_field: Some("message".to_string()),
                    }),
                    delta: None,
                    usage: None,
                    index: Some(0),
                },
                // Content delta events
                DeltaEvent {
                    event_type: "content_block_delta".to_string(),
                    message: Some(DeltaMessage {
                        id: "msg_sample123".to_string(),
                        model: "claude-3-sonnet-20240229".to_string(),
                        content: Some(vec![Content::Text {
                            text: "This is ".to_string(),
                        }]),
                        stop_reason: None,
                        stop_sequence: None,
                        role: Some(Role::Assistant),
                        type_field: Some("message".to_string()),
                    }),
                    delta: Some(Delta {
                        text: Some("This is ".to_string()),
                        stop_reason: None,
                        stop_sequence: None,
                    }),
                    usage: None,
                    index: Some(1),
                },
                // Final event with stop reason
                DeltaEvent {
                    event_type: "message_delta".to_string(),
                    message: Some(DeltaMessage {
                        id: "msg_sample123".to_string(),
                        model: "claude-3-sonnet-20240229".to_string(),
                        content: None,
                        stop_reason: Some("end_turn".to_string()),
                        stop_sequence: None,
                        role: Some(Role::Assistant),
                        type_field: Some("message".to_string()),
                    }),
                    delta: Some(Delta {
                        text: None,
                        stop_reason: Some("end_turn".to_string()),
                        stop_sequence: None,
                    }),
                    usage: Some(Usage {
                        input_tokens: 10,
                        output_tokens: 5,
                    }),
                    index: Some(4),
                },
            ]
        }
    }
    
    // Mock handler implementation
    impl MockApiHandler for StreamingMock {
        fn process_request(&self, _request: claude_rs::types::MessageRequest) -> Pin<Box<dyn Future<Output = claude_rs::types::ClaudeResult<claude_rs::types::MessageResponse>> + Send>> {
            unimplemented!("Not used in this benchmark")
        }
        
        fn process_stream_request(&self, _request: claude_rs::types::MessageRequest) -> Pin<Box<dyn Future<Output = claude_rs::types::ClaudeResult<claude_rs::types::MessageStream>> + Send>> {
            use futures::stream::iter;
            
            // Create a stream of delta events for the benchmark
            let events = Self::create_delta_events();
            let stream = iter(events.into_iter().map(Ok)).boxed();
            
            Box::pin(async move {
                Ok(Box::pin(stream) as claude_rs::types::MessageStream)
            })
        }
    }
    
    // Create client with our mock - using with_mock_api instead of with_mock
    let client = Claude::with_mock_api(
        "fake-api-key", 
        Arc::new(StreamingMock) as Arc<dyn MockApiHandler>
    );
    
    let mut group = c.benchmark_group("streaming");
    
    // Use rt.block_on instead of to_async
    group.bench_function("text_extraction", |b| {
        b.iter(|| {
            rt.block_on(async {
                let builder = client.message()
                    .user_message("Benchmark test for streaming")
                    .unwrap();
                
                let stream = builder.stream().await.unwrap();
                tokio::pin!(stream);
                
                let mut extracted_text = String::new();
                while let Some(result) = stream.next().await {
                    if let Ok(delta) = result {
                        // Use proper string borrowing
                        if let Some(d) = &delta.delta {
                            if let Some(text) = &d.text {
                                extracted_text.push_str(text);
                            }
                        } 
                        else if let Some(msg) = &delta.message {
                            if let Some(contents) = &msg.content {
                                for content in contents {
                                    if let Content::Text { text } = content {
                                        extracted_text.push_str(text);
                                    }
                                }
                            }
                        }
                    }
                }
                
                extracted_text
            })
        });
    });
    
    // Fixed reactive_text_stream to properly handle String
    group.bench_function("reactive_text_stream", |b| {
        b.iter(|| {
            rt.block_on(async {
                let builder = client.message()
                    .user_message("Benchmark test for reactive")
                    .unwrap();
                
                let reactive = client.send_reactive(builder).await.unwrap();
                let mut text_stream = reactive.text_stream();
                
                let mut result = String::new();
                while let Some(text_result) = text_stream.next().await {
                    if let Ok(chunk) = text_result {
                        result.push_str(&chunk);
                    }
                }
                
                result
            })
        });
    });
    
    group.finish();
}

// Conditionally include streaming_benchmark only when reactive feature is enabled
#[cfg(not(feature = "reactive"))]
criterion_group!(
    benches,
    client_construction_benchmark,
    message_builder_benchmark,
    domain_client_benchmark
);

#[cfg(feature = "reactive")]
criterion_group!(
    benches,
    client_construction_benchmark,
    message_builder_benchmark,
    domain_client_benchmark,
    streaming_benchmark
);

criterion_main!(benches);