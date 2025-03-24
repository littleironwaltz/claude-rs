#![cfg(feature = "reactive")]

use claude_rs::{Claude, ClaudeModel, ClaudeError};
use claude_rs::reactive::{ReactiveResponse, ReactiveResponseStatus};
use futures::StreamExt;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::mpsc;
use tokio_stream::wrappers::ReceiverStream;

// Import our mock API client
mod mock_api_client;
use mock_api_client::{MockApiClient, create_sample_delta_events, create_sample_message_response};

// Create a test helper to mock the Claude client
fn create_mock_claude() -> (Claude, Arc<MockApiClient>) {
    let mock_api = Arc::new(MockApiClient::new());
    
    // Configure the mock API with sample responses
    mock_api.add_mock(ClaudeModel::Sonnet, create_sample_message_response());
    mock_api.add_mock(ClaudeModel::Sonnet, create_sample_delta_events());
    
    // Create a Claude client with the mock API
    let client = Claude::with_mock_api(
        "test-api-key",
        mock_api_client::mock_api_to_handler(mock_api.clone()),
    ).with_model(ClaudeModel::Sonnet);
    
    (client, mock_api)
}

#[tokio::test]
async fn test_reactive_response_creation() {
    // Create a channel and stream to simulate delta events
    let (_tx, rx) = mpsc::channel(10);
    let stream = ReceiverStream::new(rx);
    
    // Create a ReactiveResponse from the stream
    let reactive = ReactiveResponse::new(stream);
    
    // Check initial state
    assert_eq!(reactive.current_text(), "");
    assert_eq!(reactive.is_complete(), false);
    assert_eq!(reactive.status(), ReactiveResponseStatus::Initializing);
    assert!(reactive.last_error().is_none());
    assert!(!reactive.has_error());
}

#[tokio::test]
async fn test_reactive_response_processing() {
    // Create mock delta events
    let events = create_sample_delta_events();
    
    // Create a channel and send events through it
    let (tx, rx) = mpsc::channel(10);
    let stream = ReceiverStream::new(rx);
    
    // Create a ReactiveResponse from the stream
    let mut reactive = ReactiveResponse::new(stream);
    
    // Initial status should be Initializing
    assert_eq!(reactive.status(), ReactiveResponseStatus::Initializing);
    
    // Clone events for use in the async task
    let events_clone = events.clone();
    
    // Spawn a task to send events
    tokio::spawn(async move {
        for event in events_clone {
            tx.send(Ok(event)).await.unwrap();
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    });
    
    // Verify that we can collect the stream
    let mut collected_events = Vec::new();
    while let Some(event) = reactive.next().await {
        collected_events.push(event);
        tokio::time::sleep(Duration::from_millis(5)).await;
    }
    
    // Should have received all events
    assert_eq!(collected_events.len(), events.len());
    
    // ReactiveResponse should be complete
    assert!(reactive.is_complete());
    assert_eq!(reactive.status(), ReactiveResponseStatus::Complete);
    
    // No errors should have occurred
    assert!(!reactive.has_error());
    assert!(reactive.last_error().is_none());
    
    // Text buffer should contain the concatenated text
    assert_eq!(
        reactive.current_text(),
        "This is a sample streaming response from the mock API."
    );
}

#[tokio::test]
async fn test_text_stream_transformation() {
    // Create mock delta events
    let events = create_sample_delta_events();
    
    // Create a channel and send events through it
    let (tx, rx) = mpsc::channel(10);
    let stream = ReceiverStream::new(rx);
    
    // Create a ReactiveResponse from the stream
    let reactive = ReactiveResponse::new(stream);
    
    // Transform to text stream
    let mut text_stream = reactive.text_stream();
    
    // Clone events for use in the async task
    let events_clone = events.clone();
    
    // Spawn a task to send events
    tokio::spawn(async move {
        for event in events_clone {
            tx.send(Ok(event)).await.unwrap();
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    });
    
    // Collect text chunks
    let mut text_chunks = Vec::new();
    while let Some(result) = text_stream.next().await {
        let chunk = result.unwrap();
        text_chunks.push(chunk);
    }
    
    // Should have received text chunks (excluding message_start and message_delta)
    assert_eq!(text_chunks.len(), 4);
    
    // Combined text should match expected output
    let combined_text = text_chunks.join("");
    assert_eq!(
        combined_text,
        "This is a sample streaming response from the mock API."
    );
}

#[tokio::test]
async fn test_reactive_client_extension() {
    // Create a mock Claude client
    let (claude, _) = create_mock_claude();
    
    // Create a message builder
    let builder = claude.message()
        .user_content("What is artificial intelligence?");
    
    // Use the reactive extension to send the message
    let reactive = claude.send_reactive(builder).await.unwrap();
    
    // Verify that the ReactiveResponse was created
    assert_eq!(reactive.is_complete(), false);
}

#[tokio::test]
async fn test_reactive_end_to_end() {
    // Create a mock Claude client
    let (claude, mock_api) = create_mock_claude();
    
    // Create a message with content
    let builder = claude.message()
        .user_content("What is artificial intelligence?");
    
    // Send the message using the reactive extension
    let mut reactive = claude.send_reactive(builder).await.unwrap();
    
    // Verify initial status
    assert_eq!(reactive.status(), ReactiveResponseStatus::Initializing);
    
    // Verify that request was recorded
    let requests = mock_api.get_request_history();
    assert_eq!(requests.len(), 1);
    
    // Wait for the response to complete
    while !reactive.is_complete() {
        reactive.next().await;
        tokio::time::sleep(Duration::from_millis(10)).await;
        
        // Status should change to Streaming or Complete
        assert!(reactive.status() == ReactiveResponseStatus::Streaming || 
                reactive.status() == ReactiveResponseStatus::Complete);
    }
    
    // Status should be Complete
    assert_eq!(reactive.status(), ReactiveResponseStatus::Complete);
    
    // No errors should have occurred
    assert!(!reactive.has_error());
    assert!(reactive.last_error().is_none());
    
    // Check the complete text
    assert_eq!(
        reactive.current_text(),
        "This is a sample streaming response from the mock API."
    );
}

#[tokio::test]
async fn test_reactive_error_handling() {
    // Create a channel and stream to simulate delta events and errors
    let (tx, rx) = mpsc::channel(10);
    let stream = ReceiverStream::new(rx);
    
    // Create a ReactiveResponse from the stream
    let mut reactive = ReactiveResponse::new(stream);
    
    // Send a few valid events, then an error
    tokio::spawn(async move {
        // First send some valid content
        let mut events = create_sample_delta_events();
        let first_event = events.remove(0); // Get the message_start event
        let content_event = events.remove(0); // Get a content event
        
        tx.send(Ok(first_event)).await.unwrap();
        tokio::time::sleep(Duration::from_millis(10)).await;
        
        tx.send(Ok(content_event)).await.unwrap();
        tokio::time::sleep(Duration::from_millis(10)).await;
        
        // Then send an error
        let error = ClaudeError::api_error(
            "Rate limit exceeded".to_string(), 
            Some(429), 
            None, 
            Some("reactive_tests.rs:224")
        );
        tx.send(Err(error)).await.unwrap();
    });
    
    // Process events including the error
    let mut events_processed = 0;
    while let Some(_event) = reactive.next().await {
        events_processed += 1;
        
        // Wait until we've processed 2 events (start + content)
        if events_processed >= 2 {
            // The next event should be an error
            if let Some(next_event) = reactive.next().await {
                assert!(next_event.is_err(), "Expected error but got {:?}", next_event);
                break;
            }
        }
    }
    
    // Status should be Error
    assert_eq!(reactive.status(), ReactiveResponseStatus::Error);
    assert!(reactive.has_error());
    
    // The error should be stored in the ReactiveResponse
    let last_error = reactive.last_error().unwrap();
    match last_error {
        ClaudeError::ApiError { status, message, .. } => {
            assert_eq!(*status, 429);
            assert!(message.contains("Rate limit exceeded"));
        },
        _ => panic!("Expected ApiError, got: {:?}", last_error),
    }
}