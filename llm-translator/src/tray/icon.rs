use tauri::{Icon, Runtime};

/// Generate tray icon based on current state
pub struct TrayIconGenerator;

impl TrayIconGenerator {
    /// Create default icon (active state)
    pub fn default_icon() -> Icon {
        // In production, load from embedded resources
        // For now, we'll create a simple colored icon
        Self::create_icon(&[0, 150, 136, 255]) // Teal color
    }
    
    /// Create inactive icon (grayed out)
    pub fn inactive_icon() -> Icon {
        Self::create_icon(&[128, 128, 128, 255]) // Gray color
    }
    
    /// Create loading icon (animated or different color)
    pub fn loading_icon() -> Icon {
        Self::create_icon(&[255, 152, 0, 255]) // Orange color
    }
    
    /// Create error icon (red)
    pub fn error_icon() -> Icon {
        Self::create_icon(&[244, 67, 54, 255]) // Red color
    }
    
    /// Create success icon (green)
    pub fn success_icon() -> Icon {
        Self::create_icon(&[76, 175, 80, 255]) // Green color
    }
    
    /// Helper to create a simple colored square icon
    fn create_icon(color: &[u8; 4]) -> Icon {
        // Create a 32x32 RGBA icon
        let size = 32;
        let mut rgba = Vec::with_capacity((size * size * 4) as usize);
        
        for y in 0..size {
            for x in 0..size {
                // Create a circle icon
                let dx = x as f32 - size as f32 / 2.0;
                let dy = y as f32 - size as f32 / 2.0;
                let distance = (dx * dx + dy * dy).sqrt();
                
                if distance < size as f32 / 2.0 - 2.0 {
                    // Inside circle
                    rgba.extend_from_slice(color);
                } else if distance < size as f32 / 2.0 {
                    // Anti-aliasing edge
                    let alpha = ((size as f32 / 2.0 - distance) * 255.0) as u8;
                    rgba.push(color[0]);
                    rgba.push(color[1]);
                    rgba.push(color[2]);
                    rgba.push(alpha);
                } else {
                    // Outside circle (transparent)
                    rgba.extend_from_slice(&[0, 0, 0, 0]);
                }
            }
        }
        
        Icon::Rgba {
            rgba,
            width: size,
            height: size,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrayIconState {
    Active,
    Inactive,
    Loading,
    Error,
    Success,
}

impl TrayIconState {
    pub fn to_icon(&self) -> Icon {
        match self {
            TrayIconState::Active => TrayIconGenerator::default_icon(),
            TrayIconState::Inactive => TrayIconGenerator::inactive_icon(),
            TrayIconState::Loading => TrayIconGenerator::loading_icon(),
            TrayIconState::Error => TrayIconGenerator::error_icon(),
            TrayIconState::Success => TrayIconGenerator::success_icon(),
        }
    }
    
    pub fn tooltip(&self) -> &'static str {
        match self {
            TrayIconState::Active => "LLM Translator - Ready",
            TrayIconState::Inactive => "LLM Translator - Disabled",
            TrayIconState::Loading => "LLM Translator - Processing...",
            TrayIconState::Error => "LLM Translator - Error",
            TrayIconState::Success => "LLM Translator - Translation Complete",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_icon_generation() {
        let icon = TrayIconGenerator::default_icon();
        match icon {
            Icon::Rgba { rgba, width, height } => {
                assert_eq!(width, 32);
                assert_eq!(height, 32);
                assert_eq!(rgba.len(), (32 * 32 * 4) as usize);
            }
            _ => panic!("Expected RGBA icon"),
        }
    }
    
    #[test]
    fn test_icon_states() {
        let states = vec![
            TrayIconState::Active,
            TrayIconState::Inactive,
            TrayIconState::Loading,
            TrayIconState::Error,
            TrayIconState::Success,
        ];
        
        for state in states {
            let icon = state.to_icon();
            assert!(matches!(icon, Icon::Rgba { .. }));
            assert!(!state.tooltip().is_empty());
        }
    }
}