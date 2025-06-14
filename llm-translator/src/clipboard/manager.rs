use arboard::Clipboard;
use std::time::{Duration, Instant};
use thiserror::Error;
use tokio::time::sleep;
use tracing::{debug, warn, error};

#[derive(Error, Debug)]
pub enum ClipboardError {
    #[error("Failed to access clipboard: {0}")]
    AccessError(String),
    
    #[error("Clipboard operation timeout")]
    Timeout,
    
    #[error("Failed to simulate key press")]
    SimulationError,
}

#[derive(Error, Debug)]
pub enum SelectionError {
    #[error("No text is selected")]
    NoSelection,
    
    #[error("Selected text is empty")]
    EmptySelection,
    
    #[error("Selected text contains only whitespace")]
    OnlyWhitespace,
    
    #[error("Clipboard operation timeout after {0}ms")]
    ClipboardTimeout(u64),
    
    #[error("Failed to access clipboard: {0}")]
    ClipboardError(String),
    
    #[error("Failed to simulate copy operation")]
    SimulationError,
}

pub struct ClipboardManager {
    clipboard: Clipboard,
    original_content: Option<String>,
    preserve_original: bool,
    timeout_ms: u64,
}

impl ClipboardManager {
    pub fn new(preserve_original: bool, timeout_ms: u64) -> Result<Self, ClipboardError> {
        let clipboard = Clipboard::new()
            .map_err(|e| ClipboardError::AccessError(e.to_string()))?;
        
        Ok(Self {
            clipboard,
            original_content: None,
            preserve_original,
            timeout_ms,
        })
    }

    /// Get currently selected text by simulating Ctrl+C
    pub async fn get_selection(&mut self) -> Result<String, SelectionError> {
        debug!("Getting text selection");
        
        // Save original clipboard content if needed
        if self.preserve_original {
            self.original_content = self.get_text().ok();
            debug!("Saved original clipboard content");
        }

        // Clear clipboard to detect new content
        self.clear()
            .map_err(|e| SelectionError::ClipboardError(e.to_string()))?;

        // Simulate Ctrl+C (platform-specific)
        self.simulate_copy().await?;

        // Wait for clipboard to be populated with timeout
        let deadline = Instant::now() + Duration::from_millis(self.timeout_ms);
        let mut check_interval = Duration::from_millis(10);
        
        loop {
            // Check if clipboard has new content
            if let Ok(text) = self.get_text() {
                if !text.is_empty() {
                    // Validate the text
                    let trimmed = text.trim();
                    
                    if trimmed.is_empty() {
                        return Err(SelectionError::OnlyWhitespace);
                    }
                    
                    debug!("Successfully captured selection: {} chars", text.len());
                    return Ok(text);
                }
            }

            // Check timeout
            if Instant::now() > deadline {
                warn!("Selection capture timeout after {}ms", self.timeout_ms);
                
                // Restore original content on timeout
                if self.preserve_original {
                    self.restore_original().await;
                }
                
                return Err(SelectionError::ClipboardTimeout(self.timeout_ms));
            }

            // Wait before next check (with exponential backoff)
            sleep(check_interval).await;
            check_interval = (check_interval * 2).min(Duration::from_millis(100));
        }
    }

    /// Set clipboard text
    pub fn set_text(&mut self, text: &str) -> Result<(), ClipboardError> {
        self.clipboard
            .set_text(text)
            .map_err(|e| ClipboardError::AccessError(e.to_string()))?;
        
        debug!("Set clipboard text: {} chars", text.len());
        Ok(())
    }

    /// Get clipboard text
    pub fn get_text(&mut self) -> Result<String, ClipboardError> {
        self.clipboard
            .get_text()
            .map_err(|e| ClipboardError::AccessError(e.to_string()))
    }

    /// Clear clipboard
    pub fn clear(&mut self) -> Result<(), ClipboardError> {
        self.set_text("")
    }

    /// Restore original clipboard content
    pub async fn restore_original(&mut self) -> Result<(), ClipboardError> {
        if let Some(ref original) = self.original_content {
            debug!("Restoring original clipboard content");
            self.set_text(original)?;
            
            // Small delay to ensure clipboard is updated
            sleep(Duration::from_millis(50)).await;
        }
        Ok(())
    }

    /// Simulate copy operation (Ctrl+C)
    async fn simulate_copy(&self) -> Result<(), SelectionError> {
        cfg_if::cfg_if! {
            if #[cfg(target_os = "windows")] {
                self.simulate_copy_windows().await
            } else if #[cfg(target_os = "macos")] {
                self.simulate_copy_macos().await
            } else {
                self.simulate_copy_linux().await
            }
        }
    }

    #[cfg(target_os = "windows")]
    async fn simulate_copy_windows(&self) -> Result<(), SelectionError> {
        use enigo::{Enigo, Key, KeyboardControllable};
        
        let mut enigo = Enigo::new();
        
        // Press Ctrl+C
        enigo.key_down(Key::Control);
        enigo.key_click(Key::Layout('c'));
        enigo.key_up(Key::Control);
        
        // Small delay to ensure key press is processed
        sleep(Duration::from_millis(50)).await;
        
        Ok(())
    }

    #[cfg(target_os = "macos")]
    async fn simulate_copy_macos(&self) -> Result<(), SelectionError> {
        use enigo::{Enigo, Key, KeyboardControllable};
        
        let mut enigo = Enigo::new();
        
        // Press Cmd+C
        enigo.key_down(Key::Meta);
        enigo.key_click(Key::Layout('c'));
        enigo.key_up(Key::Meta);
        
        // Small delay to ensure key press is processed
        sleep(Duration::from_millis(50)).await;
        
        Ok(())
    }

    #[cfg(target_os = "linux")]
    async fn simulate_copy_linux(&self) -> Result<(), SelectionError> {
        use enigo::{Enigo, Key, KeyboardControllable};
        
        let mut enigo = Enigo::new();
        
        // Press Ctrl+C
        enigo.key_down(Key::Control);
        enigo.key_click(Key::Layout('c'));
        enigo.key_up(Key::Control);
        
        // Small delay to ensure key press is processed
        sleep(Duration::from_millis(50)).await;
        
        Ok(())
    }

    /// Simulate paste operation (Ctrl+V)
    pub async fn simulate_paste(&self) -> Result<(), SelectionError> {
        cfg_if::cfg_if! {
            if #[cfg(target_os = "windows")] {
                self.simulate_paste_windows().await
            } else if #[cfg(target_os = "macos")] {
                self.simulate_paste_macos().await
            } else {
                self.simulate_paste_linux().await
            }
        }
    }

    #[cfg(target_os = "windows")]
    async fn simulate_paste_windows(&self) -> Result<(), SelectionError> {
        use enigo::{Enigo, Key, KeyboardControllable};
        
        let mut enigo = Enigo::new();
        
        // Press Ctrl+V
        enigo.key_down(Key::Control);
        enigo.key_click(Key::Layout('v'));
        enigo.key_up(Key::Control);
        
        // Small delay to ensure key press is processed
        sleep(Duration::from_millis(50)).await;
        
        Ok(())
    }

    #[cfg(target_os = "macos")]
    async fn simulate_paste_macos(&self) -> Result<(), SelectionError> {
        use enigo::{Enigo, Key, KeyboardControllable};
        
        let mut enigo = Enigo::new();
        
        // Press Cmd+V
        enigo.key_down(Key::Meta);
        enigo.key_click(Key::Layout('v'));
        enigo.key_up(Key::Meta);
        
        // Small delay to ensure key press is processed
        sleep(Duration::from_millis(50)).await;
        
        Ok(())
    }

    #[cfg(target_os = "linux")]
    async fn simulate_paste_linux(&self) -> Result<(), SelectionError> {
        use enigo::{Enigo, Key, KeyboardControllable};
        
        let mut enigo = Enigo::new();
        
        // Press Ctrl+V
        enigo.key_down(Key::Control);
        enigo.key_click(Key::Layout('v'));
        enigo.key_up(Key::Control);
        
        // Small delay to ensure key press is processed
        sleep(Duration::from_millis(50)).await;
        
        Ok(())
    }

    /// Replace selected text with new content
    pub async fn replace_selection(&mut self, new_text: &str) -> Result<(), SelectionError> {
        debug!("Replacing selection with {} chars", new_text.len());
        
        // Set new text to clipboard
        self.set_text(new_text)
            .map_err(|e| SelectionError::ClipboardError(e.to_string()))?;
        
        // Simulate paste
        self.simulate_paste().await?;
        
        // Restore original clipboard if needed
        if self.preserve_original {
            // Wait a bit to ensure paste is complete
            sleep(Duration::from_millis(100)).await;
            self.restore_original().await
                .map_err(|e| SelectionError::ClipboardError(e.to_string()))?;
        }
        
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_clipboard_manager_creation() {
        let manager = ClipboardManager::new(true, 500);
        assert!(manager.is_ok());
    }

    #[tokio::test]
    async fn test_set_and_get_text() {
        let mut manager = ClipboardManager::new(false, 500).unwrap();
        
        let test_text = "Hello, clipboard!";
        assert!(manager.set_text(test_text).is_ok());
        
        let retrieved = manager.get_text();
        assert!(retrieved.is_ok());
        assert_eq!(retrieved.unwrap(), test_text);
    }

    #[tokio::test]
    async fn test_clear_clipboard() {
        let mut manager = ClipboardManager::new(false, 500).unwrap();
        
        manager.set_text("Some text").unwrap();
        assert!(manager.clear().is_ok());
        
        let text = manager.get_text().unwrap();
        assert!(text.is_empty());
    }

    #[tokio::test]
    async fn test_preserve_original() {
        let mut manager = ClipboardManager::new(true, 500).unwrap();
        
        let original = "Original text";
        manager.set_text(original).unwrap();
        
        // Simulate saving original
        manager.original_content = Some(original.to_string());
        
        // Change clipboard
        manager.set_text("New text").unwrap();
        
        // Restore
        manager.restore_original().await.unwrap();
        
        let restored = manager.get_text().unwrap();
        assert_eq!(restored, original);
    }

    #[tokio::test]
    async fn test_empty_selection_detection() {
        let mut manager = ClipboardManager::new(false, 100).unwrap();
        
        // Clear clipboard to simulate no selection
        manager.clear().unwrap();
        
        // This should timeout since we're not actually simulating copy
        let result = manager.get_selection().await;
        assert!(result.is_err());
    }
}