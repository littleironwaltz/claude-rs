use criterion::{criterion_group, criterion_main, Criterion, BenchmarkId};
use claude_rs::utils::json_extractor::*;
use claude_rs::types::{Content, MessageResponse, Role, Usage};

fn create_test_response(content_type: &str, size: usize) -> MessageResponse {
    let content = match content_type {
        "code_block" => format!("```json\n{{\n  \"data\": \"{}\"\n}}\n```", "a".repeat(size)),
        "object" => format!("{{\n  \"data\": \"{}\"\n}}", "a".repeat(size)),
        "raw" => format!("{{\n  \"data\": \"{}\"\n}}", "a".repeat(size)),
        _ => String::new(),
    };
    
    MessageResponse {
        id: "msg_123".to_string(),
        model: "claude-3".to_string(),
        r#type: "message".to_string(),
        role: Role::Assistant,
        content: vec![Content::Text { text: content }],
        usage: Usage {
            input_tokens: 10,
            output_tokens: 10,
        },
        stop_reason: Some("end_turn".to_string()),
        stop_sequence: None,
    }
}

pub fn json_extraction_benchmark(c: &mut Criterion) {
    let mut group = c.benchmark_group("json_extraction");
    
    for content_type in ["code_block", "object", "raw"] {
        for size in [100, 1000, 10000] {
            let response = create_test_response(content_type, size);
            
            group.bench_with_input(
                BenchmarkId::new(format!("extract_{}", content_type), size),
                &response,
                |b, response| {
                    b.iter(|| {
                        let _ = extract_from_response(response);
                    });
                },
            );
        }
    }
    
    // Only benchmark the public extract_from_response function
    // since the other extraction methods are private
    for size in [100, 1000, 10000] {
        for content_type in ["code_block", "object", "raw"] {
            let response = create_test_response(content_type, size);
            group.bench_with_input(
                BenchmarkId::new(format!("extract_from_response_{}", content_type), size),
                &response,
                |b, response| {
                    b.iter(|| {
                        let _ = extract_from_response(response);
                    });
                },
            );
        }
    }
    
    group.finish();
}

criterion_group!(
    benches,
    json_extraction_benchmark
);
criterion_main!(benches);