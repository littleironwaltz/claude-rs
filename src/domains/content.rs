// Content Generation Client

use crate::client::Claude;
use crate::types::*;
use crate::domains::{DomainClient, DomainOperations, ValidationOperations, base::BaseDomainClient};
use std::collections::HashMap;
use std::sync::Arc;
use regex::Regex;

// Content Generation Client
pub struct ContentGenerationClient {
    base: BaseDomainClient,
}

#[derive(Debug, Clone)]
pub struct ContentTemplate {
    template: String,
    parameters: HashMap<String, String>,
    required_params: Vec<String>,
}

impl ContentTemplate {
    /// Create a new content template with {{parameter}} placeholders
    ///
    /// # Arguments
    /// * `template` - A string containing {{parameter}} placeholders
    ///
    /// # Returns
    /// A new ContentTemplate with extracted required parameters
    ///
    /// # Example
    /// ```
    /// use claude_rs::domains::content::ContentTemplate;
    ///
    /// let template = ContentTemplate::new("Hello, {{name}}!");
    /// ```
    pub fn new(template: impl Into<String>) -> ClaudeResult<Self> {
        let template_str = crate::utils::StringValidator::not_empty(template, "template")?;
        
        // Extract parameter names from template {{param}}
        let re = Regex::new(r"\{\{([^}]+)\}\}")
            .map_err(|e| ClaudeError::parse_error(
                format!("Regex error: {}", e), 
                None,
                Some(e), 
                Some(concat!(file!(), ":", line!()))
            ))?;
            
        let required_params = re
            .captures_iter(&template_str)
            .map(|cap| cap[1].to_string())
            .collect::<Vec<String>>();
            
        // Validate that the template contains at least one placeholder
        if required_params.is_empty() {
            return Err(ClaudeError::ValidationError(
                "Template must contain at least one {{param}} placeholder".into()
            ));
        }
            
        Ok(Self {
            template: template_str,
            parameters: HashMap::new(),
            required_params,
        })
    }
    
    /// Add a parameter value to the template
    ///
    /// # Arguments
    /// * `name` - Parameter name (without {{ }})
    /// * `value` - The value to replace the parameter with
    ///
    /// # Returns
    /// The updated template for method chaining
    ///
    /// # Example
    /// ```
    /// use claude_rs::domains::content::ContentTemplate;
    ///
    /// let template = ContentTemplate::new("Hello, {{name}}!")
    ///     .unwrap()
    ///     .with_param("name", "World")
    ///     .unwrap();
    /// ```
    pub fn with_param(mut self, name: impl Into<String>, value: impl Into<String>) -> ClaudeResult<Self> {
        let name = crate::utils::StringValidator::not_empty(name, "parameter name")?;
        let value = crate::utils::StringValidator::not_empty(value, "parameter value")?;
        
        // Validate that the parameter name is one of the required parameters
        if !self.required_params.contains(&name) {
            return Err(ClaudeError::ValidationError(
                format!("Unknown parameter '{}'. Available parameters: {}", 
                        name, self.required_params.join(", "))
            ));
        }
        
        self.parameters.insert(name, value);
        Ok(self)
    }
    
    /// Render the template by replacing all parameters with their values
    ///
    /// # Returns
    /// The rendered template with all placeholders replaced
    ///
    /// # Errors
    /// Returns an error if any required parameter is missing
    pub fn render(&self) -> ClaudeResult<String> {
        let mut result = self.template.clone();
        
        // Check if all required parameters are provided
        let missing_params: Vec<&String> = self.required_params.iter()
            .filter(|p| !self.parameters.contains_key(*p))
            .collect();
            
        if !missing_params.is_empty() {
            return Err(ClaudeError::ValidationError(
                format!("Missing required parameter(s): {}", 
                        missing_params.iter().map(|s| s.to_string()).collect::<Vec<_>>().join(", "))
            ));
        }
        
        // Replace parameters in template
        for (name, value) in &self.parameters {
            let pattern = format!("{{{{{}}}}}", name);
            result = result.replace(&pattern, value);
        }
        
        Ok(result)
    }
}

impl ContentGenerationClient {
    pub(crate) fn new(claude: Arc<Claude>) -> Self {
        Self { 
            base: BaseDomainClient::new(claude, "content_generation")
        }
    }
    
    /// Generate text content using a template
    ///
    /// The template must have all parameters filled before rendering.
    ///
    /// # Arguments
    /// * `template` - A ContentTemplate with all parameters filled
    ///
    /// # Returns
    /// The generated content as a string
    pub async fn generate_from_template(&self, template: ContentTemplate) -> ClaudeResult<String> {
        let prompt = template.render()?;
        self.text_operation(&prompt, Some(0.7), self.domain_name(), Some(1000)).await
    }
    
    /// Generate a blog post on a specific topic
    ///
    /// # Arguments
    /// * `topic` - The topic of the blog post
    /// * `tone` - Optional tone (e.g., "professional", "casual", "humorous")
    /// * `word_count` - Optional word count (default: 500)
    ///
    /// # Returns
    /// The generated blog post as a string
    pub async fn blog_post(&self, topic: impl Into<String>, tone: Option<String>, word_count: Option<u32>) -> ClaudeResult<String> {
        let topic = self.validate_string(topic, "blog topic")?;
        
        // Validate tone if provided
        let tone = if let Some(t) = tone {
            self.validate_string(t, "tone")?
        } else {
            "professional".to_string()
        };
        
        // Validate word count range
        let words = if let Some(count) = word_count {
            self.validate_range(count, 100, 5000, "word count")?
        } else {
            500
        };
        
        let prompt = format!(
            "Write a blog post about {}. Use a {} tone. The post should be approximately {} words.",
            topic, tone, words
        );
        
        self.text_operation(&prompt, Some(0.7), self.domain_name(), Some(1000)).await
    }
    
    /// Generate a product description with features
    ///
    /// # Arguments
    /// * `product` - The product name
    /// * `features` - List of product features
    /// * `target_audience` - Optional target audience (default: "general consumers")
    /// * `word_count` - Optional word count (default: 200)
    ///
    /// # Returns
    /// The generated product description as a string
    pub async fn product_description(
        &self, 
        product: impl Into<String>, 
        features: Vec<String>, 
        target_audience: Option<String>, 
        word_count: Option<u32>
    ) -> ClaudeResult<String> {
        let product = self.validate_string(product, "product")?;
        
        // Validate features
        let validated_features = if !features.is_empty() {
            self.validate_not_empty(features, "features")?
        } else {
            Vec::new()
        };
        
        // Validate target audience if provided
        let audience = if let Some(a) = target_audience {
            self.validate_string(a, "target audience")?
        } else {
            "general consumers".to_string()
        };
        
        // Validate word count range
        let words = if let Some(count) = word_count {
            self.validate_range(count, 50, 2000, "word count")?
        } else {
            200
        };
        
        let features_text = validated_features.join("\n- ");
        let features_section = if !validated_features.is_empty() {
            format!("\n\nKey features:\n- {}", features_text)
        } else {
            String::new()
        };
        
        let prompt = format!(
            "Write a compelling product description for {} targeting {}. {}. The description should be approximately {} words.",
            product, audience, features_section, words
        );
        
        self.text_operation(&prompt, Some(0.7), self.domain_name(), Some(1000)).await
    }
}

impl DomainClient for ContentGenerationClient {
    fn domain_name(&self) -> &str {
        self.base.domain_name()
    }
}

impl ValidationOperations for ContentGenerationClient {}

impl DomainOperations for ContentGenerationClient {
    fn claude(&self) -> &Claude {
        self.base.claude()
    }
}