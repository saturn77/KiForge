use crate::DemoLensApp;
use crate::ecs::UnitsResource;
use egui_lens::{ReactiveEventLogger, ReactiveEventLoggerState, LogColors};
use egui_mobius_reactive::Dynamic;
use chrono_tz::Tz;
use chrono::Local;

pub fn show_settings_panel<'a>(
    ui: &mut egui::Ui,
    app: &'a mut DemoLensApp,
    logger_state: &'a Dynamic<ReactiveEventLoggerState>,
    log_colors: &'a Dynamic<LogColors>,
) {
    let logger = ReactiveEventLogger::with_colors(logger_state, log_colors);

    ui.heading("Application Settings");
    ui.separator();
    
    // Units Section
    ui.group(|ui| {
        ui.label("Display Units");
        ui.horizontal(|ui| {
            ui.label("Global Units:");
            
            // Get current units from ECS
            let _current_unit = if let Some(units_resource) = app.ecs_world.get_resource::<UnitsResource>() {
                units_resource.display_unit
            } else {
                crate::ecs::DisplayUnit::Millimeters
            };
            
            // Track if units changed
            let mut units_changed = false;
            let prev_units = app.global_units_mils;
            
            // Update legacy global_units_mils based on selection
            if ui.selectable_value(&mut app.global_units_mils, false, "Millimeters (mm)").clicked() {
                units_changed = true;
            }
            if ui.selectable_value(&mut app.global_units_mils, true, "Mils (1/1000 inch)").clicked() {
                units_changed = true;
            }
            
            // Additional units options (disabled for now)
            ui.add_enabled(false, egui::Button::new("Micrometers (µm)"));
            ui.add_enabled(false, egui::Button::new("Nanometers (nm)"));
            
            if units_changed || prev_units != app.global_units_mils {
                // Sync to ECS
                app.sync_units_to_ecs();
                
                let units_name = if app.global_units_mils { "mils" } else { "mm" };
                logger.log_info(&format!("Changed global units to {}", units_name));
            }
        });
        ui.label("Affects: Grid spacing, board dimensions, cursor position, zoom selection");
        ui.label("Internal precision: 1 nanometer (integer-based like KiCad)");
    });
    
    ui.add_space(20.0);
    
    // Timezone Section
    ui.group(|ui| {
        ui.label("Time & Localization");
        ui.horizontal(|ui| {
            ui.label("Timezone:");
            
            // Get current timezone name or use UTC as default
            let current_tz_name = app.user_timezone.as_ref()
                .map(|s| s.as_str())
                .unwrap_or("UTC");
            
            egui::ComboBox::from_id_salt("timezone_selector")
                .selected_text(current_tz_name)
                .width(300.0)
                .show_ui(ui, |ui| {
                    // Common timezones first
                    ui.label("Common Timezones:");
                    for tz_name in &[
                        "UTC",
                        "US/Eastern", 
                        "US/Central",
                        "US/Mountain", 
                        "US/Pacific",
                        "Europe/London",
                        "Europe/Paris",
                        "Europe/Berlin",
                        "Asia/Tokyo",
                        "Asia/Shanghai",
                        "Australia/Sydney",
                    ] {
                        if ui.selectable_value(&mut app.user_timezone, Some(tz_name.to_string()), *tz_name).clicked() {
                            logger.log_info(&format!("Changed timezone to {}", tz_name));
                        }
                    }
                    
                    ui.separator();
                    ui.label("All Timezones:");
                    
                    // All timezones
                    for tz in chrono_tz::TZ_VARIANTS {
                        let tz_name = tz.name();
                        if ui.selectable_value(&mut app.user_timezone, Some(tz_name.to_string()), tz_name).clicked() {
                            logger.log_info(&format!("Changed timezone to {}", tz_name));
                        }
                    }
                });
        });
        
        ui.add_space(10.0);
        
        // Clock format selection
        ui.horizontal(|ui| {
            ui.label("Clock Format:");
            let prev_format = app.use_24_hour_clock;
            ui.selectable_value(&mut app.use_24_hour_clock, true, "24-hour (13:30:45)");
            ui.selectable_value(&mut app.use_24_hour_clock, false, "12-hour (1:30:45 PM)");
            
            if prev_format != app.use_24_hour_clock {
                let format_name = if app.use_24_hour_clock { "24-hour" } else { "12-hour" };
                logger.log_info(&format!("Changed clock format to {}", format_name));
            }
        });
        
        // Show current time in selected timezone with chosen format
        let time_format = if app.use_24_hour_clock { "%Y-%m-%d %H:%M:%S %Z" } else { "%Y-%m-%d %I:%M:%S %p %Z" };
        
        if let Some(tz_name) = &app.user_timezone {
            if let Ok(tz) = tz_name.parse::<Tz>() {
                let now = Local::now().with_timezone(&tz);
                ui.label(format!("Current time: {}", now.format(time_format)));
            }
        } else {
            let now = Local::now();
            ui.label(format!("Current time: {}", now.format(if app.use_24_hour_clock { "%Y-%m-%d %H:%M:%S" } else { "%Y-%m-%d %I:%M:%S %p" })));
        }
    });
    
    ui.add_space(20.0);
    
    // Language Section (placeholder for future)
    ui.group(|ui| {
        ui.label("Language");
        ui.horizontal(|ui| {
            ui.label("Interface Language:");
            
            egui::ComboBox::from_id_salt("language_selector")
                .selected_text("English")
                .show_ui(ui, |ui| {
                    let _ = ui.selectable_label(true, "English");
                    ui.add_enabled(false, egui::Button::new("Français (coming soon)"));
                    ui.add_enabled(false, egui::Button::new("Deutsch (coming soon)"));
                    ui.add_enabled(false, egui::Button::new("中文 (coming soon)"));
                    ui.add_enabled(false, egui::Button::new("日本語 (coming soon)"));
                });
        });
    });
    
}