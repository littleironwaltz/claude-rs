// Sentiment Analysis Client

use crate::client::Claude;
use crate::types::*;
use crate::domains::{DomainClient, DomainOperations, ValidationOperations, base::BaseDomainClient};
use serde::Deserialize;
use std::collections::HashMap;
use std::sync::Arc;

// Sentiment Analysis Client
pub struct SentimentAnalysisClient {
    base: BaseDomainClient,
}

// Results types for sentiment analysis
#[derive(Debug, Deserialize)]
pub struct SentimentResult {
    pub score: f32,
    pub sentiment: Sentiment, 
    pub aspects: HashMap<String, AspectSentiment>,
}

#[derive(Debug, Deserialize, PartialEq)]
pub enum Sentiment {
    Positive,
    Neutral,
    Negative,
}

#[derive(Debug, Deserialize)]
pub struct AspectSentiment {
    pub score: f32,
    pub sentiment: Sentiment,
    pub highlights: Vec<String>,
}

impl SentimentAnalysisClient {
    pub(crate) fn new(claude: Arc<Claude>) -> Self {
        Self { 
            base: BaseDomainClient::new(claude, "sentiment_analysis")
        }
    }
    
    /// Analyze sentiment of text with domain-specific prompt
    pub async fn analyze_text(&self, text: impl Into<String>) -> ClaudeResult<SentimentResult> {
        let text = self.validate_string(text, "text")?;
        
        // Create a prompt that will return JSON
        let prompt = format!(
            "Analyze the sentiment of the following text. Provide a JSON response with an overall sentiment score from -1.0 (very negative) to 1.0 (very positive), a sentiment category (Positive, Neutral, or Negative), and no other information.\n\nText: {}\n\nRespond with valid JSON only.", 
            text
        );
        
        self.json_operation(&prompt, Some(0.0), self.domain_name(), Some(1000)).await
    }
    
    /// Analyze sentiment with aspects
    pub async fn with_aspects(&self, text: impl Into<String>, aspects: Vec<&str>) -> ClaudeResult<SentimentResult> {
        let text = self.validate_string(text, "text")?;
        let aspects = self.validate_not_empty(aspects, "aspects")?;
        
        let aspects_str = aspects.join("\", \"");
        
        let prompt = format!(
            "Analyze the sentiment of the following text, focusing on these aspects: [\"{}\"].\n\nProvide a JSON response with an overall sentiment score from -1.0 to 1.0, a sentiment category (Positive, Neutral, or Negative), and an 'aspects' object with each aspect containing its own score, sentiment category, and key highlights.\n\nText: {}\n\nRespond with valid JSON only.",
            aspects_str, text
        );
        
        self.json_operation(&prompt, Some(0.0), self.domain_name(), Some(1000)).await
    }
}

impl DomainClient for SentimentAnalysisClient {
    fn domain_name(&self) -> &str {
        self.base.domain_name()
    }
}

impl ValidationOperations for SentimentAnalysisClient {}

impl DomainOperations for SentimentAnalysisClient {
    fn claude(&self) -> &Claude {
        self.base.claude()
    }
}