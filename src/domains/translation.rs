use std::sync::Arc;
use serde::{Deserialize, Serialize};
use crate::{Claude, ClaudeResult};
use crate::domains::{DomainClient, DomainOperations, ValidationOperations};
use super::base::BaseDomainClient;

/// Result of translation operation
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TranslationResult {
    /// The translated text
    pub translated_text: String,
    /// The source language (if detected)
    pub source_language: Option<String>,
    /// The target language
    pub target_language: String,
    /// Optional confidence score for the translation
    pub confidence: Option<f64>,
    /// Optional alternatives for specific phrases
    pub alternatives: Option<Vec<TranslationAlternative>>,
}

/// Alternative translation for a specific phrase
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TranslationAlternative {
    /// The original phrase
    pub original: String,
    /// Alternative translation
    pub alternative: String,
    /// Context or explanation for this alternative
    pub context: Option<String>,
}

/// Language detection result
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DetectedLanguage {
    /// The detected language code
    pub language: String,
    /// The confidence score (0.0-1.0)
    pub confidence: f64,
    /// The language name in English
    pub name: Option<String>,
}

/// Client for translation operations
pub struct TranslationClient {
    base: BaseDomainClient,
}

impl TranslationClient {
    /// Create a new translation client
    pub fn new(claude: Arc<Claude>) -> Self {
        Self {
            base: BaseDomainClient::new(claude, "translation"),
        }
    }
    
    /// Translate text to a target language (backward compatibility)
    pub async fn translate(
        &self,
        text: impl Into<String>,
        target_language: impl Into<String>,
        source_language: Option<impl Into<String>>,
    ) -> ClaudeResult<TranslationResult> {
        // Call the method with explicit max_tokens
        self.translate_with_tokens(text, target_language, source_language, Some(1000)).await
    }
    
    /// Translate text to a target language with explicit max_tokens
    pub async fn translate_with_tokens(
        &self,
        text: impl Into<String>,
        target_language: impl Into<String>,
        source_language: Option<impl Into<String>>,
        max_tokens: Option<u32>
    ) -> ClaudeResult<TranslationResult> {
        let text = self.validate_string(text, "text")?;
        let target_language = self.validate_string(target_language, "target_language")?;
        
        let source_prompt = if let Some(source) = source_language {
            format!("from {}", source.into())
        } else {
            "".to_string()
        };
        
        let prompt = format!(
            "Translate the following text {} to {}. Return only the JSON with the translation, no additional text.\n\nText to translate:\n{}\n",
            source_prompt,
            target_language,
            text
        );
        
        // Use the updated json_operation with max_tokens
        self.json_operation(&prompt, None, self.domain_name(), max_tokens).await
    }
    
    /// Detect the language of a text (backward compatibility)
    pub async fn detect_language(
        &self,
        text: impl Into<String>,
    ) -> ClaudeResult<DetectedLanguage> {
        // Call the method with explicit max_tokens
        self.detect_language_with_tokens(text, Some(500)).await
    }
    
    /// Detect the language of a text with explicit max_tokens
    pub async fn detect_language_with_tokens(
        &self, 
        text: impl Into<String>,
        max_tokens: Option<u32>
    ) -> ClaudeResult<DetectedLanguage> {
        let text = self.validate_string(text, "text")?;
        
        let prompt = format!(
            "Analyze the following text and determine what language it is written in. Return only a JSON object with a 'language' field containing the ISO 639-1 code, a 'name' field with the English name of the language, and a 'confidence' score between 0 and 1.\n\nText to analyze:\n{}\n",
            text
        );
        
        self.json_operation(&prompt, None, self.domain_name(), max_tokens).await
    }
    
    /// Translate text with multiple alternative translations for key phrases (backward compatibility)
    pub async fn translate_with_alternatives(
        &self,
        text: impl Into<String>,
        target_language: impl Into<String>,
        num_alternatives: Option<u8>,
    ) -> ClaudeResult<TranslationResult> {
        // Call the method with explicit max_tokens
        self.translate_with_alternatives_and_tokens(text, target_language, num_alternatives, Some(1500)).await
    }
    
    /// Translate text with multiple alternative translations for key phrases with explicit max_tokens
    pub async fn translate_with_alternatives_and_tokens(
        &self,
        text: impl Into<String>,
        target_language: impl Into<String>,
        num_alternatives: Option<u8>,
        max_tokens: Option<u32>,
    ) -> ClaudeResult<TranslationResult> {
        let text = self.validate_string(text, "text")?;
        let target_language = self.validate_string(target_language, "target_language")?;
        let num_alternatives = num_alternatives.unwrap_or(3);
        
        if num_alternatives > 5 {
            return self.domain_error("num_alternatives must be 5 or fewer");
        }
        
        let prompt = format!(
            "Translate the following text to {}. Also identify {} important phrases and provide alternative translations for them. Return only the JSON with the result.\n\nText to translate:\n{}\n",
            target_language,
            num_alternatives,
            text
        );
        
        self.json_operation(&prompt, None, self.domain_name(), max_tokens).await
    }
}

impl DomainClient for TranslationClient {
    fn domain_name(&self) -> &str {
        self.base.domain_name()
    }
}

impl ValidationOperations for TranslationClient {}

impl DomainOperations for TranslationClient {
    fn claude(&self) -> &Claude {
        self.base.claude()
    }
}

#[cfg(test)]
mod tests {
    // We'll skip tests here since they would be better implemented
    // in the tests directory using the test infrastructure
    // For now, let's just add a placeholder test
    #[test]
    fn it_works() {
        assert!(true);
    }
}