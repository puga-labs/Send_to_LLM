use std::collections::HashMap;
use global_hotkey::{hotkey::{HotKey, Code, Modifiers}, GlobalHotKeyEvent};
use cfg_if::cfg_if;
use thiserror::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Platform {
    Windows,
    MacOS,
    Linux,
}

impl Platform {
    pub fn current() -> Self {
        cfg_if! {
            if #[cfg(target_os = "windows")] {
                Platform::Windows
            } else if #[cfg(target_os = "macos")] {
                Platform::MacOS
            } else {
                Platform::Linux
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KeyCombo {
    pub modifiers: Modifiers,
    pub key: Code,
}

impl KeyCombo {
    pub fn new(modifiers: Modifiers, key: Code) -> Self {
        Self { modifiers, key }
    }

    pub fn from_string(combo: &str) -> Result<Self, HotkeyValidationError> {
        // Parse strings like "Ctrl+Shift+T"
        let parts: Vec<&str> = combo.split('+').collect();
        if parts.is_empty() {
            return Err(HotkeyValidationError::InvalidFormat(combo.to_string()));
        }

        let mut modifiers = Modifiers::empty();
        let mut key_part = None;

        for part in parts {
            match part.to_lowercase().as_str() {
                "ctrl" | "control" => modifiers |= Modifiers::CONTROL,
                "alt" => modifiers |= Modifiers::ALT,
                "shift" => modifiers |= Modifiers::SHIFT,
                "cmd" | "command" | "meta" | "win" | "windows" => modifiers |= Modifiers::META,
                _ => key_part = Some(part),
            }
        }

        let key_str = key_part.ok_or_else(|| HotkeyValidationError::InvalidFormat(combo.to_string()))?;
        let key = Self::parse_key(key_str)?;

        Ok(Self { modifiers, key })
    }

    fn parse_key(key: &str) -> Result<Code, HotkeyValidationError> {
        match key.to_uppercase().as_str() {
            "A" => Ok(Code::KeyA),
            "B" => Ok(Code::KeyB),
            "C" => Ok(Code::KeyC),
            "D" => Ok(Code::KeyD),
            "E" => Ok(Code::KeyE),
            "F" => Ok(Code::KeyF),
            "G" => Ok(Code::KeyG),
            "H" => Ok(Code::KeyH),
            "I" => Ok(Code::KeyI),
            "J" => Ok(Code::KeyJ),
            "K" => Ok(Code::KeyK),
            "L" => Ok(Code::KeyL),
            "M" => Ok(Code::KeyM),
            "N" => Ok(Code::KeyN),
            "O" => Ok(Code::KeyO),
            "P" => Ok(Code::KeyP),
            "Q" => Ok(Code::KeyQ),
            "R" => Ok(Code::KeyR),
            "S" => Ok(Code::KeyS),
            "T" => Ok(Code::KeyT),
            "U" => Ok(Code::KeyU),
            "V" => Ok(Code::KeyV),
            "W" => Ok(Code::KeyW),
            "X" => Ok(Code::KeyX),
            "Y" => Ok(Code::KeyY),
            "Z" => Ok(Code::KeyZ),
            "0" => Ok(Code::Digit0),
            "1" => Ok(Code::Digit1),
            "2" => Ok(Code::Digit2),
            "3" => Ok(Code::Digit3),
            "4" => Ok(Code::Digit4),
            "5" => Ok(Code::Digit5),
            "6" => Ok(Code::Digit6),
            "7" => Ok(Code::Digit7),
            "8" => Ok(Code::Digit8),
            "9" => Ok(Code::Digit9),
            "ESCAPE" | "ESC" => Ok(Code::Escape),
            "SPACE" => Ok(Code::Space),
            "ENTER" | "RETURN" => Ok(Code::Enter),
            "TAB" => Ok(Code::Tab),
            "DELETE" | "DEL" => Ok(Code::Delete),
            "F1" => Ok(Code::F1),
            "F2" => Ok(Code::F2),
            "F3" => Ok(Code::F3),
            "F4" => Ok(Code::F4),
            "F5" => Ok(Code::F5),
            "F6" => Ok(Code::F6),
            "F7" => Ok(Code::F7),
            "F8" => Ok(Code::F8),
            "F9" => Ok(Code::F9),
            "F10" => Ok(Code::F10),
            "F11" => Ok(Code::F11),
            "F12" => Ok(Code::F12),
            _ => Err(HotkeyValidationError::UnknownKey(key.to_string())),
        }
    }

    pub fn to_string(&self) -> String {
        let mut parts = Vec::new();
        
        if self.modifiers.contains(Modifiers::CONTROL) {
            parts.push("Ctrl");
        }
        if self.modifiers.contains(Modifiers::ALT) {
            parts.push("Alt");
        }
        if self.modifiers.contains(Modifiers::SHIFT) {
            parts.push("Shift");
        }
        if self.modifiers.contains(Modifiers::META) {
            cfg_if! {
                if #[cfg(target_os = "macos")] {
                    parts.push("Cmd");
                } else {
                    parts.push("Win");
                }
            }
        }

        parts.push(&format!("{:?}", self.key).replace("Key", ""));
        parts.join("+")
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ValidationResult {
    Valid,
    SystemConflict,
    AlreadyRegistered,
    TooSimple,
    Reserved,
}

#[derive(Error, Debug)]
pub enum HotkeyValidationError {
    #[error("Invalid hotkey format: {0}")]
    InvalidFormat(String),
    
    #[error("Unknown key: {0}")]
    UnknownKey(String),
    
    #[error("Hotkey conflicts with system shortcut")]
    SystemConflict,
    
    #[error("Hotkey is already registered")]
    AlreadyRegistered,
    
    #[error("Hotkey is too simple (needs at least 2 keys)")]
    TooSimple,
    
    #[error("Hotkey is reserved by the system")]
    Reserved,
}

pub struct HotkeyValidator {
    known_system_hotkeys: HashMap<Platform, Vec<KeyCombo>>,
    registered_hotkeys: Vec<KeyCombo>,
}

impl HotkeyValidator {
    pub fn new() -> Self {
        let mut validator = Self {
            known_system_hotkeys: HashMap::new(),
            registered_hotkeys: Vec::new(),
        };

        // Windows system hotkeys
        validator.known_system_hotkeys.insert(Platform::Windows, vec![
            KeyCombo::new(Modifiers::ALT, Code::Tab),
            KeyCombo::new(Modifiers::ALT, Code::F4),
            KeyCombo::new(Modifiers::CONTROL | Modifiers::ALT, Code::Delete),
            KeyCombo::new(Modifiers::META, Code::KeyL),
            KeyCombo::new(Modifiers::META, Code::Tab),
            KeyCombo::new(Modifiers::CONTROL | Modifiers::SHIFT, Code::Escape),
            KeyCombo::new(Modifiers::ALT, Code::Escape),
            KeyCombo::new(Modifiers::CONTROL, Code::Escape),
        ]);

        // macOS system hotkeys
        validator.known_system_hotkeys.insert(Platform::MacOS, vec![
            KeyCombo::new(Modifiers::META, Code::KeyQ),
            KeyCombo::new(Modifiers::META, Code::KeyW),
            KeyCombo::new(Modifiers::META, Code::Space),
            KeyCombo::new(Modifiers::META, Code::Tab),
            KeyCombo::new(Modifiers::META | Modifiers::SHIFT, Code::Digit3),
            KeyCombo::new(Modifiers::META | Modifiers::SHIFT, Code::Digit4),
            KeyCombo::new(Modifiers::META | Modifiers::SHIFT, Code::Digit5),
            KeyCombo::new(Modifiers::META, Code::KeyH),
            KeyCombo::new(Modifiers::META | Modifiers::ALT, Code::Escape),
        ]);

        // Linux system hotkeys (common across desktop environments)
        validator.known_system_hotkeys.insert(Platform::Linux, vec![
            KeyCombo::new(Modifiers::ALT, Code::Tab),
            KeyCombo::new(Modifiers::ALT, Code::F4),
            KeyCombo::new(Modifiers::CONTROL | Modifiers::ALT, Code::Delete),
            KeyCombo::new(Modifiers::CONTROL | Modifiers::ALT, Code::KeyL),
            KeyCombo::new(Modifiers::CONTROL | Modifiers::ALT, Code::KeyT),
            KeyCombo::new(Modifiers::ALT, Code::F2),
        ]);

        validator
    }

    pub fn validate(&self, combo: &KeyCombo) -> ValidationResult {
        let platform = Platform::current();

        // Check if it's too simple (need at least one modifier)
        if combo.modifiers.is_empty() {
            return ValidationResult::TooSimple;
        }

        // Check system hotkeys
        if let Some(system_hotkeys) = self.known_system_hotkeys.get(&platform) {
            if system_hotkeys.contains(combo) {
                return ValidationResult::SystemConflict;
            }
        }

        // Check if already registered
        if self.registered_hotkeys.contains(combo) {
            return ValidationResult::AlreadyRegistered;
        }

        // Check for specific reserved combinations
        if self.is_reserved(combo) {
            return ValidationResult::Reserved;
        }

        ValidationResult::Valid
    }

    fn is_reserved(&self, combo: &KeyCombo) -> bool {
        // Common clipboard operations
        if combo.modifiers == Modifiers::CONTROL {
            matches!(combo.key, Code::KeyC | Code::KeyV | Code::KeyX | Code::KeyA | Code::KeyZ | Code::KeyY)
        } else {
            false
        }
    }

    pub fn register(&mut self, combo: KeyCombo) -> Result<(), HotkeyValidationError> {
        match self.validate(&combo) {
            ValidationResult::Valid => {
                self.registered_hotkeys.push(combo);
                Ok(())
            }
            ValidationResult::SystemConflict => Err(HotkeyValidationError::SystemConflict),
            ValidationResult::AlreadyRegistered => Err(HotkeyValidationError::AlreadyRegistered),
            ValidationResult::TooSimple => Err(HotkeyValidationError::TooSimple),
            ValidationResult::Reserved => Err(HotkeyValidationError::Reserved),
        }
    }

    pub fn unregister(&mut self, combo: &KeyCombo) {
        self.registered_hotkeys.retain(|c| c != combo);
    }

    pub fn suggest_alternatives(&self, combo: &KeyCombo) -> Vec<KeyCombo> {
        let alternatives = vec![
            KeyCombo::new(Modifiers::CONTROL | Modifiers::SHIFT, Code::KeyT),
            KeyCombo::new(Modifiers::CONTROL | Modifiers::ALT, Code::KeyT),
            KeyCombo::new(Modifiers::ALT | Modifiers::SHIFT, Code::KeyT),
            KeyCombo::new(Modifiers::CONTROL | Modifiers::SHIFT, Code::KeyL),
            KeyCombo::new(Modifiers::CONTROL | Modifiers::ALT, Code::KeyL),
            KeyCombo::new(Modifiers::ALT | Modifiers::SHIFT, Code::KeyL),
        ];

        alternatives
            .into_iter()
            .filter(|alt| self.validate(alt) == ValidationResult::Valid)
            .collect()
    }

    pub fn clear_registered(&mut self) {
        self.registered_hotkeys.clear();
    }
}

impl Default for HotkeyValidator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_hotkey_string() {
        let combo = KeyCombo::from_string("Ctrl+Shift+T").unwrap();
        assert_eq!(combo.modifiers, Modifiers::CONTROL | Modifiers::SHIFT);
        assert_eq!(combo.key, Code::KeyT);
    }

    #[test]
    fn test_parse_invalid_hotkey() {
        assert!(KeyCombo::from_string("").is_err());
        assert!(KeyCombo::from_string("Ctrl+").is_err());
        assert!(KeyCombo::from_string("InvalidKey").is_err());
    }

    #[test]
    fn test_system_conflict_detection() {
        let validator = HotkeyValidator::new();
        
        // Alt+Tab should be a system conflict on Windows and Linux
        #[cfg(any(target_os = "windows", target_os = "linux"))]
        {
            let combo = KeyCombo::new(Modifiers::ALT, Code::Tab);
            assert_eq!(validator.validate(&combo), ValidationResult::SystemConflict);
        }
        
        // Cmd+Q should be a system conflict on macOS
        #[cfg(target_os = "macos")]
        {
            let combo = KeyCombo::new(Modifiers::META, Code::KeyQ);
            assert_eq!(validator.validate(&combo), ValidationResult::SystemConflict);
        }
    }

    #[test]
    fn test_too_simple_detection() {
        let validator = HotkeyValidator::new();
        let combo = KeyCombo::new(Modifiers::empty(), Code::KeyT);
        assert_eq!(validator.validate(&combo), ValidationResult::TooSimple);
    }

    #[test]
    fn test_reserved_detection() {
        let validator = HotkeyValidator::new();
        
        // Ctrl+C should be reserved
        let combo = KeyCombo::new(Modifiers::CONTROL, Code::KeyC);
        assert_eq!(validator.validate(&combo), ValidationResult::Reserved);
        
        // Ctrl+V should be reserved
        let combo = KeyCombo::new(Modifiers::CONTROL, Code::KeyV);
        assert_eq!(validator.validate(&combo), ValidationResult::Reserved);
    }

    #[test]
    fn test_registration() {
        let mut validator = HotkeyValidator::new();
        let combo = KeyCombo::new(Modifiers::CONTROL | Modifiers::SHIFT, Code::KeyT);
        
        // First registration should succeed
        assert!(validator.register(combo.clone()).is_ok());
        
        // Second registration of same combo should fail
        assert!(matches!(
            validator.register(combo),
            Err(HotkeyValidationError::AlreadyRegistered)
        ));
    }

    #[test]
    fn test_suggest_alternatives() {
        let validator = HotkeyValidator::new();
        let combo = KeyCombo::new(Modifiers::ALT, Code::Tab); // System hotkey
        
        let alternatives = validator.suggest_alternatives(&combo);
        assert!(!alternatives.is_empty());
        
        // All alternatives should be valid
        for alt in &alternatives {
            assert_eq!(validator.validate(alt), ValidationResult::Valid);
        }
    }

    #[test]
    fn test_hotkey_to_string() {
        let combo = KeyCombo::new(Modifiers::CONTROL | Modifiers::SHIFT, Code::KeyT);
        let string = combo.to_string();
        assert!(string.contains("Ctrl"));
        assert!(string.contains("Shift"));
        assert!(string.contains("T"));
    }
}