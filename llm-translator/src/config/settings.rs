use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::fs;
use std::collections::HashMap;
use thiserror::Error;
use validator::{Validate, ValidationError};

#[derive(Error, Debug)]
pub enum ConfigError {
    #[error("Failed to read config file: {0}")]
    ReadError(#[from] std::io::Error),
    
    #[error("Failed to parse config: {0}")]
    ParseError(#[from] toml::de::Error),
    
    #[error("Failed to serialize config: {0}")]
    SerializeError(#[from] toml::ser::Error),
    
    #[error("Config validation failed: {0}")]
    ValidationError(String),
    
    #[error("Config directory not found")]
    DirectoryNotFound,
}

#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct Config {
    #[serde(default)]
    pub general: GeneralSettings,
    
    #[serde(default)]
    pub hotkey: HotkeySettings,
    
    #[validate]
    pub api: ApiSettings,
    
    #[serde(default)]
    pub prompt: PromptSettings,
    
    #[serde(default)]
    #[validate]
    pub limits: LimitSettings,
    
    #[serde(default)]
    pub validation: ValidationSettings,
    
    #[serde(default)]
    pub behavior: BehaviorSettings,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeneralSettings {
    pub auto_start: bool,
    pub show_notifications: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HotkeySettings {
    pub translate: String,
    pub cancel: String,
    pub alternatives: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct ApiSettings {
    #[validate(url)]
    pub endpoint: String,
    
    pub model: String,
    
    #[validate(range(min = 0.0, max = 2.0))]
    pub temperature: f32,
    
    #[validate(range(min = 1, max = 10))]
    pub max_retries: u32,
    
    #[validate(range(min = 5, max = 300))]
    pub timeout_seconds: u64,
    
    #[serde(skip)]
    pub api_key: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromptSettings {
    pub active_preset: String,
    pub presets: HashMap<String, PromptPreset>,
    #[serde(default)]
    pub custom: HashMap<String, PromptPreset>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromptPreset {
    pub name: String,
    pub system: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct LimitSettings {
    #[validate(range(min = 1, max = 10000))]
    pub max_text_length: usize,
    
    #[validate(range(min = 1, max = 5000))]
    pub max_tokens_estimate: usize,
    
    #[validate(range(min = 1, max = 1000))]
    pub min_text_length: usize,
    
    #[validate(range(min = 1, max = 100))]
    pub requests_per_minute: usize,
    
    #[validate(range(min = 1, max = 10000))]
    pub requests_per_day: usize,
    
    #[validate(range(min = 100, max = 5000))]
    pub clipboard_timeout_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationSettings {
    pub allow_only_whitespace: bool,
    pub detect_binary_data: bool,
    pub trim_before_validate: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BehaviorSettings {
    pub preserve_clipboard: bool,
    pub show_length_warning: bool,
    pub auto_split_long_text: bool,
}

// Default implementations
impl Default for GeneralSettings {
    fn default() -> Self {
        Self {
            auto_start: true,
            show_notifications: true,
        }
    }
}

impl Default for HotkeySettings {
    fn default() -> Self {
        Self {
            translate: "Ctrl+Shift+T".to_string(),
            cancel: "Escape".to_string(),
            alternatives: vec![
                "Ctrl+Alt+T".to_string(),
                "Alt+Shift+T".to_string(),
            ],
        }
    }
}

impl Default for ApiSettings {
    fn default() -> Self {
        Self {
            endpoint: "https://api.openai.com/v1/chat/completions".to_string(),
            model: "gpt-4.1-nano".to_string(),
            temperature: 0.3,
            max_retries: 3,
            timeout_seconds: 30,
            api_key: None,
        }
    }
}

impl Default for PromptSettings {
    fn default() -> Self {
        let mut presets = HashMap::new();
        
        // General translation
        presets.insert("general".to_string(), PromptPreset {
            name: "Общий перевод".to_string(),
            system: "Переведи этот текст на грамотный английский язык, сохраняя смысл изначального текста. Верни только переведенный текст без каких-либо дополнений от себя.".to_string(),
        });
        
        // Twitter style
        presets.insert("twitter".to_string(), PromptPreset {
            name: "Twitter стиль".to_string(),
            system: "Translate this text to English in a casual Twitter style. Keep it concise, use common abbreviations where appropriate, and maintain the original tone. Return only the translated text.".to_string(),
        });
        
        // Formal style
        presets.insert("formal".to_string(), PromptPreset {
            name: "Официальный стиль".to_string(),
            system: "Translate this text into formal, professional English suitable for business correspondence. Maintain proper grammar and formal vocabulary. Return only the translated text.".to_string(),
        });
        
        // Academic style
        presets.insert("academic".to_string(), PromptPreset {
            name: "Академический стиль".to_string(),
            system: "Translate this text into academic English with precise terminology and formal structure. Ensure clarity and scholarly tone. Return only the translated text.".to_string(),
        });
        
        // Creative style
        presets.insert("creative".to_string(), PromptPreset {
            name: "Творческий стиль".to_string(),
            system: "Translate this text into English with creative flair, maintaining the emotional impact and artistic expression of the original. Return only the translated text.".to_string(),
        });
        
        Self {
            active_preset: "general".to_string(),
            presets,
            custom: HashMap::new(),
        }
    }
}

impl Default for LimitSettings {
    fn default() -> Self {
        Self {
            max_text_length: 5000,
            max_tokens_estimate: 1250,
            min_text_length: 1,
            requests_per_minute: 30,
            requests_per_day: 500,
            clipboard_timeout_ms: 500,
        }
    }
}

impl Default for ValidationSettings {
    fn default() -> Self {
        Self {
            allow_only_whitespace: false,
            detect_binary_data: true,
            trim_before_validate: true,
        }
    }
}

impl Default for BehaviorSettings {
    fn default() -> Self {
        Self {
            preserve_clipboard: true,
            show_length_warning: true,
            auto_split_long_text: false,
        }
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            general: GeneralSettings::default(),
            hotkey: HotkeySettings::default(),
            api: ApiSettings::default(),
            prompt: PromptSettings::default(),
            limits: LimitSettings::default(),
            validation: ValidationSettings::default(),
            behavior: BehaviorSettings::default(),
        }
    }
}

impl Config {
    pub fn load() -> Result<Self, ConfigError> {
        let config_path = Self::config_path()?;
        
        if !config_path.exists() {
            // Create default config if it doesn't exist
            let default_config = Config::default();
            default_config.save()?;
            return Ok(default_config);
        }
        
        let contents = fs::read_to_string(&config_path)?;
        let mut config: Config = toml::from_str(&contents)?;
        
        // Load API key from secure storage (keyring)
        // This will be implemented when we add keyring support
        
        // Validate the config
        config.validate().map_err(|e| {
            ConfigError::ValidationError(e.to_string())
        })?;
        
        Ok(config)
    }
    
    pub fn save(&self) -> Result<(), ConfigError> {
        let config_path = Self::config_path()?;
        
        // Ensure directory exists
        if let Some(parent) = config_path.parent() {
            fs::create_dir_all(parent)?;
        }
        
        // Don't save API key to file
        let mut config_to_save = self.clone();
        config_to_save.api.api_key = None;
        
        let contents = toml::to_string_pretty(&config_to_save)?;
        fs::write(&config_path, contents)?;
        
        Ok(())
    }
    
    pub fn config_path() -> Result<PathBuf, ConfigError> {
        let config_dir = dirs::config_dir()
            .ok_or(ConfigError::DirectoryNotFound)?;
        
        Ok(config_dir.join("llm-translator").join("config.toml"))
    }
    
    pub fn validate(&self) -> Result<(), validator::ValidationErrors> {
        <Self as Validate>::validate(self)
    }
    
    /// Get the currently active prompt
    pub fn get_active_prompt(&self) -> Option<&PromptPreset> {
        self.prompt.presets.get(&self.prompt.active_preset)
            .or_else(|| self.prompt.custom.get(&self.prompt.active_preset))
    }
    
    /// Set active prompt preset
    pub fn set_active_prompt(&mut self, preset_id: &str) -> Result<(), ConfigError> {
        if self.prompt.presets.contains_key(preset_id) || 
           self.prompt.custom.contains_key(preset_id) {
            self.prompt.active_preset = preset_id.to_string();
            Ok(())
        } else {
            Err(ConfigError::ValidationError(format!("Prompt preset '{}' not found", preset_id)))
        }
    }
    
    /// Add custom prompt preset
    pub fn add_custom_prompt(&mut self, id: String, preset: PromptPreset) {
        self.prompt.custom.insert(id, preset);
    }
    
    /// Remove custom prompt preset
    pub fn remove_custom_prompt(&mut self, id: &str) -> Option<PromptPreset> {
        self.prompt.custom.remove(id)
    }
    
    /// Get all available prompts (presets + custom)
    pub fn get_all_prompts(&self) -> Vec<(String, &PromptPreset)> {
        let mut prompts = Vec::new();
        
        // Add presets
        for (id, preset) in &self.prompt.presets {
            prompts.push((id.clone(), preset));
        }
        
        // Add custom
        for (id, preset) in &self.prompt.custom {
            prompts.push((id.clone(), preset));
        }
        
        prompts
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    
    #[test]
    fn test_default_config() {
        let config = Config::default();
        assert!(config.general.auto_start);
        assert!(config.general.show_notifications);
        assert_eq!(config.hotkey.translate, "Ctrl+Shift+T");
        assert_eq!(config.limits.max_text_length, 5000);
    }
    
    #[test]
    fn test_config_validation() {
        let mut config = Config::default();
        
        // Valid config should pass
        assert!(config.validate().is_ok());
        
        // Invalid temperature
        config.api.temperature = 3.0;
        assert!(config.validate().is_err());
        config.api.temperature = 0.3;
        
        // Invalid text length
        config.limits.max_text_length = 0;
        assert!(config.validate().is_err());
        config.limits.max_text_length = 5000;
        
        // Invalid URL
        config.api.endpoint = "not-a-url".to_string();
        assert!(config.validate().is_err());
    }
    
    #[test]
    fn test_config_serialization() {
        let config = Config::default();
        let serialized = toml::to_string(&config).unwrap();
        let deserialized: Config = toml::from_str(&serialized).unwrap();
        
        assert_eq!(config.general.auto_start, deserialized.general.auto_start);
        assert_eq!(config.hotkey.translate, deserialized.hotkey.translate);
        assert_eq!(config.limits.max_text_length, deserialized.limits.max_text_length);
    }
    
    #[test]
    fn test_partial_config() {
        // Test that partial configs work with defaults
        let toml_str = r#"
[api]
endpoint = "https://api.openai.com/v1/chat/completions"
model = "gpt-4"
temperature = 0.5
max_retries = 3
timeout_seconds = 30
"#;
        
        let config: Config = toml::from_str(toml_str).unwrap();
        
        // Should have default values for missing sections
        assert_eq!(config.general.auto_start, true);
        assert_eq!(config.hotkey.translate, "Ctrl+Shift+T");
        
        // Should have provided values for api section
        assert_eq!(config.api.model, "gpt-4");
        assert_eq!(config.api.temperature, 0.5);
    }
}