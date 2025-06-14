use std::sync::Arc;
use tauri::{
    AppHandle, Manager, Runtime,
    tray::{TrayIcon, TrayIconBuilder, TrayIconEvent},
    menu::MenuEvent,
};
use tokio::sync::{mpsc, RwLock};
use tracing::{debug, error, info};

use super::icon::{TrayIconState, TrayIconGenerator};
use super::menu::TrayMenuBuilder;
use crate::config::Config;

#[derive(Debug, Clone)]
pub enum TrayEvent {
    Toggle,
    OpenSettings,
    ChangeStyle(String),
    ResetStatistics,
    ShowHelp(String),
    Quit,
}

pub struct TrayHandler<R: Runtime> {
    app: AppHandle<R>,
    tray: TrayIcon<R>,
    config: Arc<RwLock<Config>>,
    event_sender: mpsc::Sender<TrayEvent>,
    current_state: Arc<RwLock<TrayIconState>>,
}

impl<R: Runtime> TrayHandler<R> {
    pub fn new(
        app: AppHandle<R>,
        config: Arc<RwLock<Config>>,
        event_sender: mpsc::Sender<TrayEvent>,
    ) -> tauri::Result<Self> {
        // Create tray icon
        let tray = TrayIconBuilder::new()
            .icon(TrayIconGenerator::default_icon())
            .tooltip("LLM Translator")
            .menu(&TrayMenuBuilder::build(&app)?)
            .on_menu_event(|app, event| {
                Self::handle_menu_event(app, event);
            })
            .on_tray_icon_event(|tray, event| {
                Self::handle_tray_event(tray, event);
            })
            .build(&app)?;
        
        Ok(Self {
            app,
            tray,
            config,
            event_sender,
            current_state: Arc::new(RwLock::new(TrayIconState::Active)),
        })
    }
    
    /// Update tray icon state
    pub async fn set_state(&self, state: TrayIconState) -> tauri::Result<()> {
        let mut current = self.current_state.write().await;
        if *current != state {
            *current = state;
            drop(current);
            
            self.tray.set_icon(Some(state.to_icon()))?;
            self.tray.set_tooltip(Some(state.tooltip()))?;
            
            // Update menu status
            let status_text = match state {
                TrayIconState::Active => "Status: Active",
                TrayIconState::Inactive => "Status: Inactive",
                TrayIconState::Loading => "Status: Processing...",
                TrayIconState::Error => "Status: Error",
                TrayIconState::Success => "Status: Success",
            };
            
            TrayMenuBuilder::update_status(&self.app, status_text, state != TrayIconState::Inactive)?;
        }
        
        Ok(())
    }
    
    /// Update statistics in menu
    pub async fn update_statistics(
        &self,
        today_count: usize,
        remaining_minute: usize,
        remaining_day: usize,
    ) -> tauri::Result<()> {
        TrayMenuBuilder::update_statistics(
            &self.app,
            today_count,
            remaining_minute,
            remaining_day,
        )
    }
    
    /// Set active translation style
    pub async fn set_active_style(&self, style_id: &str) -> tauri::Result<()> {
        TrayMenuBuilder::set_active_style(&self.app, style_id)
    }
    
    /// Handle menu events
    fn handle_menu_event(app: &AppHandle<R>, event: MenuEvent) {
        let event_id = event.id().0.as_str();
        debug!("Menu event: {}", event_id);
        
        // Get the event sender from app state
        if let Some(sender) = app.try_state::<mpsc::Sender<TrayEvent>>() {
            let tray_event = match event_id {
                "toggle" => Some(TrayEvent::Toggle),
                "settings" => Some(TrayEvent::OpenSettings),
                "style_general" => Some(TrayEvent::ChangeStyle("general".to_string())),
                "style_twitter" => Some(TrayEvent::ChangeStyle("twitter".to_string())),
                "style_formal" => Some(TrayEvent::ChangeStyle("formal".to_string())),
                "style_academic" => Some(TrayEvent::ChangeStyle("academic".to_string())),
                "style_creative" => Some(TrayEvent::ChangeStyle("creative".to_string())),
                "stats_reset" => Some(TrayEvent::ResetStatistics),
                "help_docs" => Some(TrayEvent::ShowHelp("docs".to_string())),
                "help_shortcuts" => Some(TrayEvent::ShowHelp("shortcuts".to_string())),
                "help_about" => Some(TrayEvent::ShowHelp("about".to_string())),
                "quit" => Some(TrayEvent::Quit),
                _ => None,
            };
            
            if let Some(event) = tray_event {
                let sender = sender.inner().clone();
                tauri::async_runtime::spawn(async move {
                    if let Err(e) = sender.send(event).await {
                        error!("Failed to send tray event: {}", e);
                    }
                });
            }
        }
    }
    
    /// Handle tray icon events
    fn handle_tray_event(_tray: &TrayIcon<R>, event: TrayIconEvent) {
        match event {
            TrayIconEvent::Click { 
                id: _, 
                position: _, 
                rect: _, 
                button, 
                button_state: _ 
            } => {
                debug!("Tray clicked with {:?}", button);
                // On left click, could show a quick status window
                // On right click, the menu is shown automatically
            }
            TrayIconEvent::Enter { id: _, position: _, rect: _ } => {
                // Could show a tooltip or status
            }
            TrayIconEvent::Leave { id: _, position: _, rect: _ } => {
                // Hide any temporary UI
            }
            _ => {}
        }
    }
    
    /// Flash the tray icon to get user attention
    pub async fn flash_attention(&self, duration_ms: u64) -> tauri::Result<()> {
        let original_state = *self.current_state.read().await;
        
        // Flash between current and success/error state
        for _ in 0..3 {
            self.set_state(TrayIconState::Success).await?;
            tokio::time::sleep(tokio::time::Duration::from_millis(duration_ms / 6)).await;
            self.set_state(original_state).await?;
            tokio::time::sleep(tokio::time::Duration::from_millis(duration_ms / 6)).await;
        }
        
        Ok(())
    }
    
    /// Show a notification balloon (if supported by the OS)
    pub fn show_notification(&self, title: &str, message: &str) -> tauri::Result<()> {
        self.app
            .notification()
            .builder()
            .title(title)
            .body(message)
            .show()?;
            
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tray_event_creation() {
        let events = vec![
            TrayEvent::Toggle,
            TrayEvent::OpenSettings,
            TrayEvent::ChangeStyle("general".to_string()),
            TrayEvent::ResetStatistics,
            TrayEvent::ShowHelp("docs".to_string()),
            TrayEvent::Quit,
        ];
        
        for event in events {
            // Just ensure they can be created and cloned
            let _cloned = event.clone();
        }
    }
}