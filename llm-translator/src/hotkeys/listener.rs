use global_hotkey::{GlobalHotKeyManager, GlobalHotKeyEvent, HotKeyState};
use global_hotkey::hotkey::{HotKey, Code, Modifiers};
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};
use std::collections::HashMap;
use tracing::{debug, info, warn, error};
use thiserror::Error;

use super::validator::{HotkeyValidator, KeyCombo, ValidationResult};

#[derive(Error, Debug)]
pub enum HotkeyError {
    #[error("Failed to register hotkey: {0}")]
    RegistrationError(String),
    
    #[error("Hotkey conflict detected")]
    ConflictError,
    
    #[error("Invalid hotkey format: {0}")]
    InvalidFormat(String),
    
    #[error("Hotkey manager error: {0}")]
    ManagerError(String),
}

#[derive(Debug, Clone)]
pub enum HotkeyEvent {
    Translate,
    Cancel,
    Custom(String),
}

pub struct HotkeyListener {
    manager: GlobalHotKeyManager,
    validator: Arc<RwLock<HotkeyValidator>>,
    registered_hotkeys: Arc<RwLock<HashMap<u32, (HotKey, HotkeyEvent)>>>,
    event_sender: mpsc::Sender<HotkeyEvent>,
    fallback_hotkeys: Vec<KeyCombo>,
}

impl HotkeyListener {
    pub fn new(event_sender: mpsc::Sender<HotkeyEvent>) -> Result<Self, HotkeyError> {
        let manager = GlobalHotKeyManager::new()
            .map_err(|e| HotkeyError::ManagerError(e.to_string()))?;
        
        Ok(Self {
            manager,
            validator: Arc::new(RwLock::new(HotkeyValidator::new())),
            registered_hotkeys: Arc::new(RwLock::new(HashMap::new())),
            event_sender,
            fallback_hotkeys: vec![
                KeyCombo::new(Modifiers::CONTROL | Modifiers::SHIFT, Code::KeyT),
                KeyCombo::new(Modifiers::CONTROL | Modifiers::ALT, Code::KeyT),
                KeyCombo::new(Modifiers::ALT | Modifiers::SHIFT, Code::KeyT),
            ],
        })
    }

    /// Register a hotkey with automatic conflict resolution
    pub async fn register_hotkey(
        &self,
        combo_str: &str,
        event: HotkeyEvent,
        use_fallback: bool,
    ) -> Result<HotKey, HotkeyError> {
        let combo = KeyCombo::from_string(combo_str)
            .map_err(|e| HotkeyError::InvalidFormat(e.to_string()))?;
        
        // Try to register the requested hotkey
        match self.try_register(&combo, event.clone()).await {
            Ok(hotkey) => {
                info!("Successfully registered hotkey: {}", combo_str);
                Ok(hotkey)
            }
            Err(e) if use_fallback => {
                warn!("Failed to register {}: {}. Trying fallbacks...", combo_str, e);
                self.register_with_fallback(event).await
            }
            Err(e) => Err(e),
        }
    }

    /// Try to register a specific hotkey
    async fn try_register(
        &self,
        combo: &KeyCombo,
        event: HotkeyEvent,
    ) -> Result<HotKey, HotkeyError> {
        // Create the hotkey first
        let hotkey = HotKey::new(Some(combo.modifiers), combo.key);
        
        // Acquire both locks to ensure atomicity
        let mut validator = self.validator.write().await;
        let mut registered = self.registered_hotkeys.write().await;
        
        // Validate the hotkey while holding the lock
        match validator.validate(combo) {
            ValidationResult::Valid => {},
            ValidationResult::SystemConflict => {
                return Err(HotkeyError::ConflictError);
            }
            ValidationResult::AlreadyRegistered => {
                return Err(HotkeyError::ConflictError);
            }
            ValidationResult::TooSimple => {
                return Err(HotkeyError::InvalidFormat("Hotkey needs at least one modifier".to_string()));
            }
            ValidationResult::Reserved => {
                return Err(HotkeyError::InvalidFormat("This key combination is reserved".to_string()));
            }
        }
        
        // Try to register with the system
        if let Err(e) = self.manager.register(hotkey) {
            return Err(HotkeyError::RegistrationError(e.to_string()));
        }
        
        // Update validator state - this should not fail since we already validated
        if let Err(_) = validator.register(combo.clone()) {
            // Rollback system registration
            let _ = self.manager.unregister(hotkey);
            return Err(HotkeyError::ConflictError);
        }
        
        // Update registered hotkeys
        registered.insert(hotkey.id(), (hotkey, event));
        
        debug!("Registered hotkey: {:?} with id: {}", combo, hotkey.id());
        
        Ok(hotkey)
    }

    /// Try fallback hotkeys if the primary one fails
    async fn register_with_fallback(
        &self,
        event: HotkeyEvent,
    ) -> Result<HotKey, HotkeyError> {
        for fallback in &self.fallback_hotkeys {
            match self.try_register(fallback, event.clone()).await {
                Ok(hotkey) => {
                    info!("Registered fallback hotkey: {}", fallback.to_string());
                    return Ok(hotkey);
                }
                Err(e) => {
                    debug!("Fallback {} failed: {}", fallback.to_string(), e);
                    continue;
                }
            }
        }
        
        Err(HotkeyError::ConflictError)
    }

    /// Unregister a hotkey
    pub async fn unregister_hotkey(&self, hotkey: &HotKey) -> Result<(), HotkeyError> {
        // Acquire locks first for atomicity
        let mut validator = self.validator.write().await;
        let mut registered = self.registered_hotkeys.write().await;
        
        // Get the combo info before removing
        let combo = if let Some((hk, _)) = registered.get(&hotkey.id()) {
            KeyCombo::new(
                hk.mods.unwrap_or(Modifiers::empty()),
                hk.key
            )
        } else {
            return Ok(()); // Already unregistered
        };
        
        // Unregister from system
        self.manager.unregister(hotkey)
            .map_err(|e| HotkeyError::ManagerError(e.to_string()))?;
        
        // Remove from our records
        registered.remove(&hotkey.id());
        validator.unregister(&combo);
        
        info!("Unregistered hotkey with id: {}", hotkey.id());
        
        Ok(())
    }

    /// Unregister all hotkeys
    pub async fn unregister_all(&self) -> Result<(), HotkeyError> {
        let registered = self.registered_hotkeys.read().await;
        let hotkeys: Vec<HotKey> = registered.values().map(|(hk, _)| *hk).collect();
        drop(registered);
        
        for hotkey in hotkeys {
            self.unregister_hotkey(&hotkey).await?;
        }
        
        let mut validator = self.validator.write().await;
        validator.clear_registered();
        
        Ok(())
    }

    /// Start listening for hotkey events
    pub async fn start_listening(self: Arc<Self>) {
        let receiver = GlobalHotKeyEvent::receiver();
        let registered = self.registered_hotkeys.clone();
        let sender = self.event_sender.clone();
        
        // Use blocking thread for the receiver since it's a blocking operation
        std::thread::spawn(move || {
            info!("Hotkey listener started");
            
            loop {
                // This blocks until an event is available, avoiding busy-wait
                match receiver.recv() {
                    Ok(event) => {
                        if event.state == HotKeyState::Pressed {
                            // Use blocking read since we're in a sync context
                            let registered_lock = registered.blocking_read();
                            
                            if let Some((_, hotkey_event)) = registered_lock.get(&event.id) {
                                debug!("Hotkey pressed: {:?}", hotkey_event);
                                
                                let event_clone = hotkey_event.clone();
                                let sender_clone = sender.clone();
                                
                                // Send event asynchronously
                                tokio::spawn(async move {
                                    if let Err(e) = sender_clone.send(event_clone).await {
                                        error!("Failed to send hotkey event: {}", e);
                                    }
                                });
                            }
                        }
                    }
                    Err(e) => {
                        error!("Failed to receive hotkey event: {:?}", e);
                        // Continue listening even on error
                    }
                }
            }
        });
    }

    /// Get currently registered hotkeys
    pub async fn get_registered(&self) -> Vec<(String, HotkeyEvent)> {
        let registered = self.registered_hotkeys.read().await;
        
        registered.values()
            .map(|(hotkey, event)| {
                let combo = KeyCombo::new(
                    hotkey.mods.unwrap_or(Modifiers::empty()),
                    hotkey.key
                );
                (combo.to_string(), event.clone())
            })
            .collect()
    }

    /// Check if a hotkey string would conflict
    pub async fn check_conflict(&self, combo_str: &str) -> Result<ValidationResult, HotkeyError> {
        let combo = KeyCombo::from_string(combo_str)
            .map_err(|e| HotkeyError::InvalidFormat(e.to_string()))?;
        
        let validator = self.validator.read().await;
        Ok(validator.validate(&combo))
    }

    /// Get suggestions for alternative hotkeys
    pub async fn get_suggestions(&self, combo_str: &str) -> Result<Vec<String>, HotkeyError> {
        let combo = KeyCombo::from_string(combo_str)
            .map_err(|e| HotkeyError::InvalidFormat(e.to_string()))?;
        
        let validator = self.validator.read().await;
        let suggestions = validator.suggest_alternatives(&combo);
        
        Ok(suggestions.into_iter().map(|c| c.to_string()).collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::sync::mpsc;

    #[tokio::test]
    async fn test_hotkey_listener_creation() {
        let (sender, _receiver) = mpsc::channel(10);
        let listener = HotkeyListener::new(sender);
        assert!(listener.is_ok());
    }

    #[tokio::test]
    async fn test_conflict_detection() {
        let (sender, _receiver) = mpsc::channel(10);
        let listener = HotkeyListener::new(sender).unwrap();
        
        // Test system hotkey conflict
        #[cfg(target_os = "windows")]
        {
            let result = listener.check_conflict("Alt+Tab").await;
            assert!(matches!(result, Ok(ValidationResult::SystemConflict)));
        }
        
        // Test valid hotkey
        let result = listener.check_conflict("Ctrl+Shift+T").await;
        assert!(matches!(result, Ok(ValidationResult::Valid)));
    }

    #[tokio::test]
    async fn test_get_suggestions() {
        let (sender, _receiver) = mpsc::channel(10);
        let listener = HotkeyListener::new(sender).unwrap();
        
        let suggestions = listener.get_suggestions("Alt+Tab").await.unwrap();
        assert!(!suggestions.is_empty());
        
        // All suggestions should be valid
        for suggestion in suggestions {
            let result = listener.check_conflict(&suggestion).await.unwrap();
            assert_eq!(result, ValidationResult::Valid);
        }
    }
}