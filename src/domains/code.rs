// Code Assistance Client

use crate::client::Claude;
use crate::types::*;
use crate::domains::{DomainClient, DomainOperations, ValidationOperations, base::BaseDomainClient};
use serde::Deserialize;
use std::sync::Arc;

// Code Assistance Client
pub struct CodeAssistanceClient {
    base: BaseDomainClient,
}

#[derive(Debug, Deserialize)]
pub struct CodeAnalysis {
    pub issues: Vec<CodeIssue>,
    pub suggestions: Vec<CodeSuggestion>,
    pub complexity_score: u32,
    pub summary: String,
}

#[derive(Debug, Deserialize)]
pub struct CodeIssue {
    pub line: Option<u32>,
    pub severity: IssueSeverity,
    pub description: String,
    pub code: Option<String>,
}

#[derive(Debug, Deserialize, PartialEq, Eq)]
pub enum IssueSeverity {
    Critical,
    High,
    Medium,
    Low,
    Info,
}

#[derive(Debug, Deserialize)]
pub struct CodeSuggestion {
    pub description: String,
    pub original_code: Option<String>,
    pub suggested_code: Option<String>,
    pub explanation: String,
}

impl CodeAssistanceClient {
    pub(crate) fn new(claude: Arc<Claude>) -> Self {
        Self { 
            base: BaseDomainClient::new(claude, "code_assistance")
        }
    }
    
    /// Analyze code for issues and suggestions
    pub async fn analyze_code(&self, code: impl Into<String>, language: impl Into<String>) -> ClaudeResult<CodeAnalysis> {
        let code = self.validate_string(code, "code")?;
        let language = self.validate_string(language, "language")?;
        
        let prompt = format!(
            "Analyze this {} code for potential issues, bugs, and improvements. Provide your analysis in JSON format with 'issues' (array of issues with 'line', 'severity', 'description'), 'suggestions' (array of improvement suggestions with 'description', 'original_code', 'suggested_code', 'explanation'), 'complexity_score' (1-10), and 'summary'.\n\nCode:\n```{}\n{}\n```\n\nRespond with valid JSON only.",
            language, language, code
        );
        
        self.json_operation(&prompt, Some(0.0), self.domain_name(), Some(1000)).await
    }
    
    /// Generate documentation for code
    pub async fn generate_docs(
        &self, 
        code: impl Into<String>, 
        language: impl Into<String>, 
        doc_style: Option<String>
    ) -> ClaudeResult<String> {
        let code = self.validate_string(code, "code")?;
        let language = self.validate_string(language, "language")?;
        let style = doc_style.unwrap_or_else(|| "standard".to_string());
        
        let prompt = format!(
            "Generate {} documentation for this {} code. The documentation should explain the purpose, parameters, return values, and provide examples where appropriate.\n\nCode:\n```{}\n{}\n```",
            style, language, language, code
        );
        
        self.text_operation(&prompt, None, self.domain_name(), Some(1500)).await
    }
    
    /// Refactor code
    pub async fn refactor_code(
        &self, 
        code: impl Into<String>, 
        language: impl Into<String>, 
        goal: impl Into<String>
    ) -> ClaudeResult<String> {
        let code = self.validate_string(code, "code")?;
        let language = self.validate_string(language, "language")?;
        let goal = self.validate_string(goal, "goal")?;
        
        let prompt = format!(
            "Refactor this {} code to {}. Provide the refactored code with explanations of the changes made.\n\nOriginal Code:\n```{}\n{}\n```",
            language, goal, language, code
        );
        
        self.text_operation(&prompt, None, self.domain_name(), Some(1500)).await
    }
}

impl DomainClient for CodeAssistanceClient {
    fn domain_name(&self) -> &str {
        self.base.domain_name()
    }
}

impl ValidationOperations for CodeAssistanceClient {}

impl DomainOperations for CodeAssistanceClient {
    fn claude(&self) -> &Claude {
        self.base.claude()
    }
}