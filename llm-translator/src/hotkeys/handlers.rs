use tokio::sync::mpsc;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{info, error, debug};

use super::listener::HotkeyEvent;
use crate::clipboard::{ClipboardManager, SelectionError};
use crate::validation::{TextValidator, TextValidationError};
use crate::config::Config;

#[derive(Debug, Clone)]
pub enum TranslationEvent {
    TranslateRequest(String),
    CancelTranslation,
    ValidationError(String),
    ClipboardError(String),
}

pub struct HotkeyHandler {
    config: Arc<RwLock<Config>>,
    event_sender: mpsc::Sender<TranslationEvent>,
    clipboard_manager: Arc<RwLock<ClipboardManager>>,
    text_validator: Arc<TextValidator>,
}

impl HotkeyHandler {
    pub fn new(
        config: Arc<RwLock<Config>>,
        event_sender: mpsc::Sender<TranslationEvent>,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let config_lock = config.blocking_read();
        
        // Create clipboard manager
        let clipboard_manager = ClipboardManager::new(
            config_lock.behavior.preserve_clipboard,
            config_lock.limits.clipboard_timeout_ms,
        )?;
        
        // Create text validator
        let text_validator = TextValidator::new(
            config_lock.limits.max_text_length,
            config_lock.limits.max_tokens_estimate,
            config_lock.limits.min_text_length,
        )
        .with_whitespace_allowed(config_lock.validation.allow_only_whitespace)
        .with_binary_detection(config_lock.validation.detect_binary_data)
        .with_trim(config_lock.validation.trim_before_validate);
        
        drop(config_lock);
        
        Ok(Self {
            config,
            event_sender,
            clipboard_manager: Arc::new(RwLock::new(clipboard_manager)),
            text_validator: Arc::new(text_validator),
        })
    }

    /// Handle incoming hotkey events
    pub async fn handle_event(&self, event: HotkeyEvent) {
        match event {
            HotkeyEvent::Translate => {
                info!("Translation hotkey pressed");
                self.handle_translate().await;
            }
            HotkeyEvent::Cancel => {
                info!("Cancel hotkey pressed");
                self.handle_cancel().await;
            }
            HotkeyEvent::Custom(name) => {
                debug!("Custom hotkey pressed: {}", name);
                // Handle custom hotkeys if needed
            }
        }
    }

    /// Handle translation request
    async fn handle_translate(&self) {
        // Get selected text
        let text = match self.capture_selection().await {
            Ok(text) => text,
            Err(e) => {
                error!("Failed to capture selection: {}", e);
                
                let message = match e {
                    SelectionError::NoSelection | SelectionError::EmptySelection => {
                        "Please select some text first"
                    }
                    SelectionError::OnlyWhitespace => {
                        "Selected text contains only whitespace"
                    }
                    SelectionError::ClipboardTimeout(_) => {
                        "Failed to capture selection - please try again"
                    }
                    _ => "Failed to access clipboard",
                };
                
                self.send_event(TranslationEvent::ClipboardError(message.to_string())).await;
                return;
            }
        };

        // Validate text
        match self.text_validator.validate(&text) {
            Ok(validated_text) => {
                info!("Text validated successfully: {} chars", validated_text.len());
                self.send_event(TranslationEvent::TranslateRequest(validated_text)).await;
            }
            Err(e) => {
                error!("Text validation failed: {}", e);
                
                let message = match e {
                    TextValidationError::TooLong { length, max } => {
                        format!("Text is too long: {} characters (max: {})", length, max)
                    }
                    TextValidationError::TooShort { length, min } => {
                        format!("Text is too short: {} characters (min: {})", length, min)
                    }
                    TextValidationError::TooManyTokens { estimated, max } => {
                        format!("Text has too many tokens: ~{} (max: {})", estimated, max)
                    }
                    TextValidationError::OnlyWhitespace => {
                        "Text contains only whitespace".to_string()
                    }
                    TextValidationError::ContainsBinary => {
                        "Text contains binary data".to_string()
                    }
                    TextValidationError::Empty => {
                        "No text selected".to_string()
                    }
                };
                
                self.send_event(TranslationEvent::ValidationError(message)).await;
            }
        }
    }

    /// Handle cancel request
    async fn handle_cancel(&self) {
        self.send_event(TranslationEvent::CancelTranslation).await;
    }

    /// Capture selected text from clipboard
    async fn capture_selection(&self) -> Result<String, SelectionError> {
        let mut clipboard = self.clipboard_manager.write().await;
        clipboard.get_selection().await
    }

    /// Send event to the main application
    async fn send_event(&self, event: TranslationEvent) {
        if let Err(e) = self.event_sender.send(event).await {
            error!("Failed to send translation event: {}", e);
        }
    }

    /// Replace selected text with translation
    pub async fn replace_selection(&self, translated_text: &str) -> Result<(), SelectionError> {
        let mut clipboard = self.clipboard_manager.write().await;
        clipboard.replace_selection(translated_text).await
    }

    /// Update configuration
    pub async fn update_config(&mut self, new_config: Config) {
        *self.config.write().await = new_config.clone();
        
        // Recreate components with new config
        if let Ok(new_clipboard) = ClipboardManager::new(
            new_config.behavior.preserve_clipboard,
            new_config.limits.clipboard_timeout_ms,
        ) {
            *self.clipboard_manager.write().await = new_clipboard;
        }
        
        self.text_validator = Arc::new(
            TextValidator::new(
                new_config.limits.max_text_length,
                new_config.limits.max_tokens_estimate,
                new_config.limits.min_text_length,
            )
            .with_whitespace_allowed(new_config.validation.allow_only_whitespace)
            .with_binary_detection(new_config.validation.detect_binary_data)
            .with_trim(new_config.validation.trim_before_validate)
        );
        
        info!("Hotkey handler configuration updated");
    }

    /// Check if text should be auto-split
    pub async fn should_auto_split(&self, text: &str) -> bool {
        let config = self.config.read().await;
        config.behavior.auto_split_long_text && text.len() > config.limits.max_text_length
    }

    /// Split text into chunks
    pub fn split_text(&self, text: &str) -> Vec<String> {
        self.text_validator.split_text(text)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{Config, BehaviorSettings, LimitSettings, ValidationSettings};

    async fn create_test_handler() -> (HotkeyHandler, mpsc::Receiver<TranslationEvent>) {
        let config = Arc::new(RwLock::new(Config::default()));
        let (sender, receiver) = mpsc::channel(10);
        
        let handler = HotkeyHandler::new(config, sender).unwrap();
        (handler, receiver)
    }

    #[tokio::test]
    async fn test_handler_creation() {
        let (_, _) = create_test_handler().await;
        // Should create without errors
    }

    #[tokio::test]
    async fn test_handle_translate_event() {
        let (handler, mut receiver) = create_test_handler().await;
        
        // This will fail because we can't actually capture selection in tests
        handler.handle_event(HotkeyEvent::Translate).await;
        
        // Should receive an error event
        let event = receiver.recv().await.unwrap();
        match event {
            TranslationEvent::ClipboardError(_) => {
                // Expected in test environment
            }
            _ => panic!("Expected clipboard error"),
        }
    }

    #[tokio::test]
    async fn test_handle_cancel_event() {
        let (handler, mut receiver) = create_test_handler().await;
        
        handler.handle_event(HotkeyEvent::Cancel).await;
        
        let event = receiver.recv().await.unwrap();
        assert!(matches!(event, TranslationEvent::CancelTranslation));
    }

    #[tokio::test]
    async fn test_text_splitting() {
        let (handler, _) = create_test_handler().await;
        
        let long_text = "a".repeat(10000);
        let chunks = handler.split_text(&long_text);
        
        assert!(chunks.len() > 1);
        for chunk in chunks {
            assert!(chunk.len() <= 5000); // Default max length
        }
    }

    #[tokio::test]
    async fn test_config_update() {
        let (mut handler, _) = create_test_handler().await;
        
        let mut new_config = Config::default();
        new_config.behavior.preserve_clipboard = false;
        new_config.limits.max_text_length = 1000;
        
        handler.update_config(new_config.clone()).await;
        
        let config = handler.config.read().await;
        assert_eq!(config.behavior.preserve_clipboard, false);
        assert_eq!(config.limits.max_text_length, 1000);
    }
}