use tauri::{
    menu::{Menu, MenuItem, PredefinedMenuItem, Submenu},
    AppHandle, Manager, Runtime,
};

#[derive(Clone)]
pub struct TrayMenuBuilder;

impl TrayMenuBuilder {
    pub fn build<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<Menu<R>> {
        let menu = Menu::new(app)?;
        
        // Status item (disabled, just for display)
        let status = MenuItem::with_id(app, "status", "Status: Active", true, None::<&str>)?;
        
        // Separator
        let separator1 = PredefinedMenuItem::separator(app)?;
        
        // Translation settings submenu
        let translation_menu = Submenu::with_items(
            app,
            "Translation Style",
            true,
            &[
                &MenuItem::with_id(app, "style_general", "General", true, None::<&str>)?,
                &MenuItem::with_id(app, "style_twitter", "Twitter", true, None::<&str>)?,
                &MenuItem::with_id(app, "style_formal", "Formal", true, None::<&str>)?,
                &MenuItem::with_id(app, "style_academic", "Academic", true, None::<&str>)?,
                &MenuItem::with_id(app, "style_creative", "Creative", true, None::<&str>)?,
            ],
        )?;
        
        // Main actions
        let toggle = MenuItem::with_id(app, "toggle", "Disable Translation", true, None::<&str>)?;
        let settings = MenuItem::with_id(app, "settings", "Settings...", true, None::<&str>)?;
        
        // Separator
        let separator2 = PredefinedMenuItem::separator(app)?;
        
        // Statistics submenu
        let stats_menu = Submenu::with_items(
            app,
            "Statistics",
            true,
            &[
                &MenuItem::with_id(app, "stats_today", "Today: 0 translations", false, None::<&str>)?,
                &MenuItem::with_id(app, "stats_remaining", "Remaining: 0/0", false, None::<&str>)?,
                &MenuItem::with_id(app, "stats_reset", "Reset Statistics", true, None::<&str>)?,
            ],
        )?;
        
        // Help submenu
        let help_menu = Submenu::with_items(
            app,
            "Help",
            true,
            &[
                &MenuItem::with_id(app, "help_docs", "Documentation", true, None::<&str>)?,
                &MenuItem::with_id(app, "help_shortcuts", "Keyboard Shortcuts", true, None::<&str>)?,
                &MenuItem::with_id(app, "help_about", "About", true, None::<&str>)?,
            ],
        )?;
        
        // Separator
        let separator3 = PredefinedMenuItem::separator(app)?;
        
        // Quit
        let quit = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
        
        // Build the menu
        menu.append(&status)?;
        menu.append(&separator1)?;
        menu.append(&translation_menu)?;
        menu.append(&toggle)?;
        menu.append(&settings)?;
        menu.append(&separator2)?;
        menu.append(&stats_menu)?;
        menu.append(&help_menu)?;
        menu.append(&separator3)?;
        menu.append(&quit)?;
        
        Ok(menu)
    }
    
    pub fn update_status<R: Runtime>(
        app: &AppHandle<R>,
        status_text: &str,
        enabled: bool,
    ) -> tauri::Result<()> {
        if let Some(item) = app.menu().get("status") {
            item.as_menuitem().unwrap().set_text(status_text)?;
        }
        
        if let Some(item) = app.menu().get("toggle") {
            let toggle_text = if enabled { "Disable Translation" } else { "Enable Translation" };
            item.as_menuitem().unwrap().set_text(toggle_text)?;
        }
        
        Ok(())
    }
    
    pub fn update_statistics<R: Runtime>(
        app: &AppHandle<R>,
        today_count: usize,
        remaining_minute: usize,
        remaining_day: usize,
    ) -> tauri::Result<()> {
        if let Some(item) = app.menu().get("stats_today") {
            item.as_menuitem().unwrap().set_text(&format!("Today: {} translations", today_count))?;
        }
        
        if let Some(item) = app.menu().get("stats_remaining") {
            item.as_menuitem().unwrap().set_text(&format!(
                "Remaining: {}/min, {}/day", 
                remaining_minute, 
                remaining_day
            ))?;
        }
        
        Ok(())
    }
    
    pub fn set_active_style<R: Runtime>(
        app: &AppHandle<R>,
        style_id: &str,
    ) -> tauri::Result<()> {
        // Reset all style checkmarks
        let styles = ["style_general", "style_twitter", "style_formal", "style_academic", "style_creative"];
        
        for style in &styles {
            if let Some(item) = app.menu().get(style) {
                let is_active = *style == style_id;
                // In Tauri 2.0, we would use set_checked if available
                // For now, we'll update the text to indicate selection
                let menu_item = item.as_menuitem().unwrap();
                let text = menu_item.text()?;
                if is_active && !text.starts_with("✓ ") {
                    menu_item.set_text(&format!("✓ {}", text))?;
                } else if !is_active && text.starts_with("✓ ") {
                    menu_item.set_text(&text[2..])?;
                }
            }
        }
        
        Ok(())
    }
}