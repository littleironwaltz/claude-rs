// Entity Extraction Client

use crate::client::Claude;
use crate::types::*;
use crate::domains::{DomainClient, DomainOperations, ValidationOperations, base::BaseDomainClient};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;

// Entity Extraction Client
pub struct EntityExtractionClient {
    base: BaseDomainClient,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Entity {
    pub text: String,
    pub entity_type: EntityType,
    pub start_idx: Option<usize>,
    pub end_idx: Option<usize>,
    pub confidence: f32,
    pub metadata: Option<HashMap<String, String>>,
}

#[derive(Debug, Deserialize, Serialize, PartialEq, Eq, Hash)]
pub enum EntityType {
    Person,
    Organization,
    Location,
    Date,
    Time,
    Money,
    Percent,
    Product,
    Event,
    WorkOfArt,
    Law,
    Language,
    Custom(String),
}

impl EntityExtractionClient {
    pub(crate) fn new(claude: Arc<Claude>) -> Self {
        Self { 
            base: BaseDomainClient::new(claude, "entity_extraction")
        }
    }
    
    /// Extract entities from text
    pub async fn extract_from_text<T: Into<String>>(&self, text: T) -> ClaudeResult<Vec<Entity>> {
        let text = self.validate_string(text, "text")?;
        
        let prompt = format!(
            "Extract named entities from the following text. Provide your response as a JSON array of entity objects, each with 'text', 'entity_type', 'start_idx' (if applicable), 'end_idx' (if applicable), and 'confidence' (between 0 and 1).\n\nEntity types to identify: Person, Organization, Location, Date, Time, Money, Percent, Product, Event, WorkOfArt, Law, Language.\n\nText: {}\n\nRespond with valid JSON only.",
            text
        );
        
        self.json_operation(&prompt, Some(0.0), self.domain_name(), Some(1000)).await
    }
    
    /// Extract specific entity types
    pub async fn with_types<T: Into<String>>(&self, text: T, types: Vec<EntityType>) -> ClaudeResult<Vec<Entity>> {
        let text = self.validate_string(text, "text")?;
        let types = self.validate_not_empty(types, "entity types")?;
        
        // Convert entity types to strings
        let type_strings: Vec<String> = types.iter().map(|t| match t {
            EntityType::Person => "Person".to_string(),
            EntityType::Organization => "Organization".to_string(),
            EntityType::Location => "Location".to_string(),
            EntityType::Date => "Date".to_string(),
            EntityType::Time => "Time".to_string(),
            EntityType::Money => "Money".to_string(),
            EntityType::Percent => "Percent".to_string(),
            EntityType::Product => "Product".to_string(),
            EntityType::Event => "Event".to_string(),
            EntityType::WorkOfArt => "WorkOfArt".to_string(),
            EntityType::Law => "Law".to_string(),
            EntityType::Language => "Language".to_string(),
            EntityType::Custom(s) => s.clone(),
        }).collect();
        
        let types_str = type_strings.join("\", \"");
        
        let prompt = format!(
            "Extract only these entity types [\"{}\"] from the following text. Provide your response as a JSON array of entity objects, each with 'text', 'entity_type', 'start_idx' (if applicable), 'end_idx' (if applicable), and 'confidence' (between 0 and 1).\n\nText: {}\n\nRespond with valid JSON only.",
            types_str, text
        );
        
        self.json_operation(&prompt, Some(0.0), self.domain_name(), Some(1000)).await
    }
    
    /// Filter returned entities by type
    pub fn of_type<'a>(&self, entities: &'a [Entity], entity_type: &EntityType) -> Vec<&'a Entity> {
        entities.iter().filter(move |e| &e.entity_type == entity_type).collect()
    }
}

impl DomainClient for EntityExtractionClient {
    fn domain_name(&self) -> &str {
        self.base.domain_name()
    }
}

impl ValidationOperations for EntityExtractionClient {}

impl DomainOperations for EntityExtractionClient {
    fn claude(&self) -> &Claude {
        self.base.claude()
    }
}