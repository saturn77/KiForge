use crate::DemoLensApp;
use crate::ui;
use crate::ecs::{UnitsResource, mm_to_nm, nm_to_mm, mils_to_nm, nm_to_mils};

use eframe::emath::{Rect, Vec2};
use eframe::epaint::Color32;
use egui::{Painter, Pos2, Stroke};
use egui_dock::{SurfaceIndex, NodeIndex};
use serde::{Serialize, Deserialize};

use egui_lens::ReactiveEventLogger;
use gerber_viewer::{
    draw_crosshair,
    draw_marker, ViewState
};
use crate::drc_operations::types::Position;
use crate::display::manager::ToPosition;
use nalgebra::Vector2;

const MARKER_RADIUS: f32 = 6.0;

/// Define the tabs for the DockArea
#[derive(Clone, Serialize, Deserialize)]
pub enum TabKind {
    ViewSettings,
    DRC,
    GerberView,
    EventLog,
    Project,
    Settings,
    BOM,
}

pub struct TabParams<'a> {
    pub app: &'a mut DemoLensApp,
}

/// Tab container struct for DockArea
#[derive(Clone, Serialize, Deserialize)]
pub struct Tab {
    pub kind: TabKind,
    #[serde(skip)]
    #[allow(dead_code)]
    pub surface: Option<SurfaceIndex>,
    #[serde(skip)]
    #[allow(dead_code)]
    pub node: Option<NodeIndex>,
}

impl Tab {
    /// Helper to get units resource from app
    fn get_units(app: &DemoLensApp) -> &UnitsResource {
        app.ecs_world.get_resource::<UnitsResource>()
            .expect("UnitsResource should exist")
    }
    
    pub fn new(kind: TabKind, surface: SurfaceIndex, node: NodeIndex) -> Self {
        Self {
            kind,
            surface: Some(surface),
            node: Some(node),
        }
    }

    pub fn title(&self) -> String {
        match self.kind {
            TabKind::ViewSettings => "View Settings".to_string(),
            TabKind::DRC => "DRC".to_string(),
            TabKind::GerberView => "Gerber View".to_string(),
            TabKind::EventLog => "Event Log".to_string(),
            TabKind::Project => "Project".to_string(),
            TabKind::Settings => "Settings".to_string(),
            TabKind::BOM => "BOM".to_string(),
        }
    }

    pub fn content(&self, ui: &mut egui::Ui, params: &mut TabParams<'_>) {
        match self.kind {
            TabKind::ViewSettings => {
                ui.vertical(|ui| {
                    let logger_state_clone = params.app.logger_state.clone();
                    let log_colors_clone = params.app.log_colors.clone();
                    
                    ui.heading("Layer Controls");
                    ui.separator();
                    ui::show_layers_panel(ui, params.app, &logger_state_clone, &log_colors_clone);
                });
            }
            TabKind::DRC => {
                let logger_state_clone = params.app.logger_state.clone();
                let log_colors_clone = params.app.log_colors.clone();
                ui::show_drc_panel(ui, params.app, &logger_state_clone, &log_colors_clone);
            }
            TabKind::GerberView => {
                self.render_gerber_view(ui, params.app);
            }
            TabKind::EventLog => {
                let logger = ReactiveEventLogger::with_colors(&params.app.logger_state, &params.app.log_colors);
                logger.show(ui);
            }
            TabKind::Project => {
                let logger_state_clone = params.app.logger_state.clone();
                let log_colors_clone = params.app.log_colors.clone();
                ui::show_project_panel(ui, params.app, &logger_state_clone, &log_colors_clone);
            }
            TabKind::Settings => {
                let logger_state_clone = params.app.logger_state.clone();
                let log_colors_clone = params.app.log_colors.clone();
                ui::show_settings_panel(ui, params.app, &logger_state_clone, &log_colors_clone);
            }
            TabKind::BOM => {
                let logger_state_clone = params.app.logger_state.clone();
                let log_colors_clone = params.app.log_colors.clone();
                ui::show_bom_panel(ui, params.app, &logger_state_clone, &log_colors_clone);
            }
        }
    }

    fn render_gerber_view(&self, ui: &mut egui::Ui, app: &mut DemoLensApp) {
        // Render top controls
        render_controls(ui, app);
        ui.separator();
        
        // Set up viewport and handle interactions
        let (viewport, response) = setup_viewport(ui, app);
        handle_viewport_interactions(ui, app, &viewport, &response);
        
        // Render the gerber layers and overlays
        render_gerber_content(ui, app, &viewport);
    }
}

fn render_controls(ui: &mut egui::Ui, app: &mut DemoLensApp) {
    ui.vertical(|ui| {
        // First row: Main view controls
        ui.horizontal(|ui| {
            render_quadrant_controls(ui, app);
            ui.separator();
            render_layer_controls(ui, app);
            ui.separator();
            render_transform_controls(ui, app);
        });
        
        ui.add_space(4.0); // Small gap between rows
        
        // Second row: Measurement and grid tools
        ui.horizontal(|ui| {
            render_zoom_display(ui, app);
            ui.separator();
            render_ruler_controls(ui, app);
            ui.separator();
            render_grid_controls(ui, app);
        });
    });
}

fn render_quadrant_controls(ui: &mut egui::Ui, app: &mut DemoLensApp) {
    if ui.checkbox(&mut app.display_manager.quadrant_view_enabled, "Quadrant View").clicked() {
        crate::ecs::mark_coordinates_dirty_ecs(&mut app.ecs_world);
        app.needs_initial_view = true;
    }
    
    if app.display_manager.quadrant_view_enabled {
        ui.separator();
        ui.label("Spacing:");
        
        // Get units from ECS
        let units_resource = Tab::get_units(app);
        
        let (mut spacing_value, units_suffix, conversion_factor) = if units_resource.is_mils() {
            let spacing_nm = mm_to_nm(app.display_manager.quadrant_offset_magnitude as f32);
            (nm_to_mils(spacing_nm), "mils", 0.0254)
        } else {
            (app.display_manager.quadrant_offset_magnitude as f32, "mm", 1.0)
        };
        
        let speed = if units_resource.is_mils() { 10.0 } else { 1.0 };
        let max_range = if units_resource.is_mils() { 20000.0 } else { 500.0 };
        
        if ui.add(egui::DragValue::new(&mut spacing_value)
            .suffix(units_suffix)
            .speed(speed)
            .range(0.0..=max_range))
            .changed() 
        {
            let spacing_mm = spacing_value * conversion_factor;
            app.display_manager.set_quadrant_offset_magnitude(spacing_mm as f64);
            crate::ecs::mark_coordinates_dirty_ecs(&mut app.ecs_world);
        }
        
        ui.separator();
        
        if ui.button("📷 Export Layers as PNG").clicked() {
            let logger_state = app.logger_state.clone();
            let log_colors = app.log_colors.clone();
            let logger = ReactiveEventLogger::with_colors(&logger_state, &log_colors);
            crate::ui::orientation_panel::export_quadrant_layers_to_png(app, &logger);
        }
    }
}

fn render_layer_controls(ui: &mut egui::Ui, app: &mut DemoLensApp) {
    let flip_text = if app.display_manager.showing_top { "🔄 Flip to Bottom (F)" } else { "🔄 Flip to Top (F)" };
    if ui.button(flip_text).clicked() {
        app.display_manager.showing_top = !app.display_manager.showing_top;
        
        // Auto-toggle layer visibility based on flip state (using ECS)
        for layer_type in crate::ecs::LayerType::all() {
            let visible = match layer_type {
                crate::ecs::LayerType::Copper(1) |
                crate::ecs::LayerType::Silkscreen(crate::ecs::Side::Top) |
                crate::ecs::LayerType::Soldermask(crate::ecs::Side::Top) |
                crate::ecs::LayerType::Paste(crate::ecs::Side::Top) => {
                    app.display_manager.showing_top
                },
                crate::ecs::LayerType::Copper(_) => {
                    !app.display_manager.showing_top
                },
                crate::ecs::LayerType::Silkscreen(crate::ecs::Side::Bottom) |
                crate::ecs::LayerType::Soldermask(crate::ecs::Side::Bottom) |
                crate::ecs::LayerType::Paste(crate::ecs::Side::Bottom) => {
                    !app.display_manager.showing_top
                },
                crate::ecs::LayerType::MechanicalOutline => {
                    // Leave outline visibility unchanged, get current state from ECS
                    crate::ecs::get_layer_visibility(&mut app.ecs_world, layer_type)
                }
            };
            crate::ecs::set_layer_visibility(&mut app.ecs_world, layer_type, visible);
        }
        
        crate::ecs::mark_coordinates_dirty_ecs(&mut app.ecs_world);
    }
}

fn render_transform_controls(ui: &mut egui::Ui, app: &mut DemoLensApp) {
    // Rotate button
    if ui.button("🔄 Rotate (R)").clicked() {
        app.rotation_degrees = (app.rotation_degrees + 90.0) % 360.0;
        
        // Don't reset view - just mark coordinates as dirty to update rotation
        // This keeps the view centered on the current origin
        crate::ecs::mark_coordinates_dirty_ecs(&mut app.ecs_world);
        
        let logger_state = app.logger_state.clone();
        let log_colors = app.log_colors.clone();
        let logger = ReactiveEventLogger::with_colors(&logger_state, &log_colors);
        logger.log_custom(
            crate::project::constants::LOG_TYPE_ROTATION, 
            &format!("Rotated to {:.0}°", app.rotation_degrees)
        );
    }
    
    // ECS Rendering is now the default and only mode (gerber-viewer 0.2.0 compatible)
    ui.label("🔥 ECS Rendering (v0.2.0)");
    
    // Mirror buttons
    let x_mirror_text = if app.display_manager.mirroring.x { "↔️ X Mirror ✓" } else { "↔️ X Mirror" };
    if ui.button(x_mirror_text).clicked() {
        app.display_manager.mirroring.x = !app.display_manager.mirroring.x;
        // Don't reset custom origin, just mark coordinates as dirty
        crate::ecs::mark_coordinates_dirty_ecs(&mut app.ecs_world);
        
        let logger_state = app.logger_state.clone();
        let log_colors = app.log_colors.clone();
        let logger = ReactiveEventLogger::with_colors(&logger_state, &log_colors);
        logger.log_custom(
            crate::project::constants::LOG_TYPE_MIRROR,
            &format!("X mirroring {}", if app.display_manager.mirroring.x { "enabled" } else { "disabled" })
        );
    }
    
    let y_mirror_text = if app.display_manager.mirroring.y { "↕️ Y Mirror ✓" } else { "↕️ Y Mirror" };
    if ui.button(y_mirror_text).clicked() {
        app.display_manager.mirroring.y = !app.display_manager.mirroring.y;
        // Don't reset custom origin, just mark coordinates as dirty
        crate::ecs::mark_coordinates_dirty_ecs(&mut app.ecs_world);
        
        let logger_state = app.logger_state.clone();
        let log_colors = app.log_colors.clone();
        let logger = ReactiveEventLogger::with_colors(&logger_state, &log_colors);
        logger.log_custom(
            crate::project::constants::LOG_TYPE_MIRROR,
            &format!("Y mirroring {}", if app.display_manager.mirroring.y { "enabled" } else { "disabled" })
        );
    }
    
    ui.separator();
    
    // Origin setting button
    let origin_set = app.display_manager.design_offset.x != 0.0 || app.display_manager.design_offset.y != 0.0;
    if origin_set {
        if ui.button("🎯 Reset Origin").clicked() {
            app.display_manager.design_offset = crate::display::VectorOffset { x: 0.0, y: 0.0 };
            app.origin_has_been_set = false;
            
            // Force view refresh to properly center coordinates at the new origin
            app.needs_initial_view = true;
            
            // Mark coordinates as dirty to force refresh
            crate::ecs::mark_coordinates_dirty_ecs(&mut app.ecs_world);
            
            let logger_state = app.logger_state.clone();
            let log_colors = app.log_colors.clone();
            let logger = ReactiveEventLogger::with_colors(&logger_state, &log_colors);
            logger.log_info("Reset origin to (0, 0) - view recentered");
        }
    } else {
        if ui.button("🎯 Set Origin").clicked() {
            app.setting_origin_mode = true;
            
            let logger_state = app.logger_state.clone();
            let log_colors = app.log_colors.clone();
            let logger = ReactiveEventLogger::with_colors(&logger_state, &log_colors);
            logger.log_info("Click on the PCB to set the origin");
        }
    }
}

fn render_grid_controls(ui: &mut egui::Ui, app: &mut DemoLensApp) {
    ui.label("Grid:");
    let grid_spacings_mils = [100.0, 50.0, 25.0, 10.0, 5.0, 2.0, 1.0];
    let grid_spacings_mm = [2.54, 1.27, 0.635, 0.254, 0.127, 0.0508, 0.0254];
    
    let is_mils = {
        let units_resource = Tab::get_units(app);
        units_resource.is_mils()
    };
    
    let spacings = if is_mils {
        &grid_spacings_mils[..]
    } else {
        &grid_spacings_mm[..]
    };
    
    // Find current selection
    let mut current_spacing_display = "Custom".to_string();
    for &spacing in spacings {
        let spacing_mm = if is_mils { 
            nm_to_mm(mils_to_nm(spacing)) 
        } else { 
            spacing 
        };
        if (app.grid_settings.spacing_mm - spacing_mm).abs() < 0.001 {
            current_spacing_display = if is_mils {
                format!("{} mils", spacing as i32)
            } else {
                format!("{:.3} mm", spacing)
            };
            break;
        }
    }
    
    egui::ComboBox::from_label("")
        .selected_text(current_spacing_display)
        .show_ui(ui, |ui| {
            for &spacing in spacings {
                let spacing_mm = if is_mils { 
                    nm_to_mm(mils_to_nm(spacing)) 
                } else { 
                    spacing 
                };
                let label = if is_mils {
                    format!("{} mils", spacing as i32)
                } else {
                    format!("{:.3} mm", spacing)
                };
                if ui.selectable_label(false, label).clicked() {
                    app.grid_settings.spacing_mm = spacing_mm;
                }
            }
        });
    
    ui.separator();
    
    // Grid dot size slider
    ui.label("Dot Size:");
    ui.add(egui::Slider::new(&mut app.grid_settings.dot_size, 0.5..=5.0).suffix("px"));
    
    ui.separator();
    
    // Enterprise feature: Snap to Grid
    ui.checkbox(&mut app.grid_settings.snap_enabled, "🧲 Snap to Grid");
}

fn render_ruler_controls(ui: &mut egui::Ui, app: &mut DemoLensApp) {
    ui.label("📏 Ruler Tool:");
    
    let ruler_button_text = if app.ruler_active { "📏 Ruler ✓" } else { "📏 Ruler" };
    if ui.button(ruler_button_text).clicked() {
        app.ruler_active = !app.ruler_active;
        if !app.ruler_active {
            // Clear ruler when deactivated
            app.ruler_start = None;
            app.ruler_end = None;
        }
    }
    
    // Show ruler measurement if active and both points set
    if app.ruler_active {
        if let (Some(start), Some(end)) = (app.ruler_start, app.ruler_end) {
            let dx = end.x - start.x;
            let dy = end.y - start.y;
            let distance = (dx * dx + dy * dy).sqrt();
            
            let units_resource = Tab::get_units(app);
            if units_resource.is_mils() {
                let distance_nm = mm_to_nm(distance as f32);
                let distance_mils = nm_to_mils(distance_nm);
                let dx_nm = mm_to_nm(dx.abs() as f32);
                let dy_nm = mm_to_nm(dy.abs() as f32);
                ui.label(format!("📏 Distance: {:.2} mils", distance_mils));
                ui.label(format!("📐 ΔX: {:.3} mils, ΔY: {:.3} mils", nm_to_mils(dx_nm), nm_to_mils(dy_nm)));
            } else {
                ui.label(format!("📏 Distance: {:.3} mm", distance));
                ui.label(format!("📐 ΔX: {:.3} mm, ΔY: {:.3} mm", dx.abs(), dy.abs()));
            }
        } else if app.ruler_start.is_some() {
            ui.label("Click second point to complete measurement");
        } else {
            ui.label("Click first point to start measurement (or press M to toggle)");
        }
    }
    // Show latched measurement if not in active measurement mode
    else if let (Some(start), Some(end)) = (app.latched_measurement_start, app.latched_measurement_end) {
        let dx = end.x - start.x;
        let dy = end.y - start.y;
        let distance = (dx * dx + dy * dy).sqrt();
        
        let units_resource = Tab::get_units(app);
        if units_resource.is_mils() {
            let distance_nm = mm_to_nm(distance as f32);
            let distance_mils = nm_to_mils(distance_nm);
            let dx_nm = mm_to_nm(dx.abs() as f32);
            let dy_nm = mm_to_nm(dy.abs() as f32);
            let dx_mils = nm_to_mils(dx_nm);
            let dy_mils = nm_to_mils(dy_nm);
            ui.label(egui::RichText::new(format!("📏 Distance: {:.2} mils", distance_mils)).color(egui::Color32::LIGHT_GRAY));
            // Use more precision for deltas to avoid showing 0.00 for small values
            ui.label(egui::RichText::new(format!("📐 ΔX: {:.3} mils, ΔY: {:.3} mils", dx_mils, dy_mils)).color(egui::Color32::LIGHT_GRAY));
        } else {
            ui.label(egui::RichText::new(format!("📏 Distance: {:.3} mm", distance)).color(egui::Color32::LIGHT_GRAY));
            ui.label(egui::RichText::new(format!("📐 ΔX: {:.3} mm, ΔY: {:.3} mm", dx.abs(), dy.abs())).color(egui::Color32::LIGHT_GRAY));
        }
        ui.label(egui::RichText::new("(Previous measurement - press M to start new)").color(egui::Color32::GRAY).italics());
    }
}

fn setup_viewport(ui: &mut egui::Ui, app: &mut DemoLensApp) -> (Rect, egui::Response) {
    ui.ctx().request_repaint();
    
    let available_size = ui.available_size();
    let size = egui::Vec2::new(
        available_size.x.max(100.0),
        available_size.y.max(100.0)
    );
    
    let response = ui.allocate_response(size, egui::Sense::click_and_drag());
    let viewport = response.rect;
    
    // Handle double-click to center view (but maintain custom origin)
    if response.double_clicked() {
        // Only reset the view, don't change the custom origin (design_offset)
        app.needs_initial_view = true;
        
        let logger_state = app.logger_state.clone();
        let log_colors = app.log_colors.clone();
        let logger = ReactiveEventLogger::with_colors(&logger_state, &log_colors);
        logger.log_info("Centered view (double-click)");
    }
    
    (viewport, response)
}

fn handle_viewport_interactions(ui: &mut egui::Ui, app: &mut DemoLensApp, viewport: &Rect, response: &egui::Response) {
    let mouse_pos_screen = ui.input(|i| i.pointer.hover_pos());
    
    // Handle zoom window
    handle_zoom_window(ui, app, viewport, mouse_pos_screen, response);
    
    // Handle mouse wheel zoom
    handle_mouse_wheel_zoom(ui, app, viewport, response);
    
    // Update UI state if not dragging zoom window
    if !app.zoom_window_dragging {
        app.ui_state.update(ui, viewport, response, &mut app.view_state);
        
        let viewport_center = viewport.center();
        
        // Calculate actual origin position based on custom origin if set
        let design_offset = &app.display_manager.design_offset;
        if design_offset.x != 0.0 || design_offset.y != 0.0 {
            // Custom origin is set - convert to screen position
            app.ui_state.origin_screen_pos = app.view_state.gerber_to_screen_coords(
                Vector2::from(design_offset.clone()).to_position().to_point2()
            );
        } else {
            // No custom origin - use viewport center
            app.ui_state.origin_screen_pos = viewport_center;
        }
        
        app.ui_state.center_screen_pos = viewport_center;
        
        // Update cursor coordinates using raw transform (not affected by design_offset)
        if let Some(cursor_pos) = ui.input(|i| i.pointer.hover_pos()) {
            // Use the original gerber coordinate system for origin setting
            let raw_gerber_pos = app.view_state.screen_to_gerber_coords(cursor_pos);
            app.ui_state.cursor_gerber_coords = Some(raw_gerber_pos);
        }
        
        // Show visual feedback when in origin setting mode
        if app.setting_origin_mode {
            ui.ctx().set_cursor_icon(egui::CursorIcon::Crosshair);
            
            // Draw preview text at cursor
            if let Some(mouse_pos) = ui.input(|i| i.pointer.hover_pos()) {
                let painter = ui.painter();
                painter.text(
                    mouse_pos + Vec2::new(20.0, -20.0),
                    egui::Align2::LEFT_BOTTOM,
                    "Click to set origin",
                    egui::FontId::default(),
                    Color32::YELLOW,
                );
            }
        }
        
        // Show visual feedback when in measurement mode
        if app.ruler_active && !app.setting_origin_mode {
            ui.ctx().set_cursor_icon(egui::CursorIcon::Crosshair);
            
            // Draw preview text at cursor
            if let Some(mouse_pos) = ui.input(|i| i.pointer.hover_pos()) {
                let painter = ui.painter();
                
                // Draw preview text
                let text = if app.ruler_start.is_some() && app.ruler_end.is_none() {
                    "Click second point to complete measurement"
                } else {
                    "Click to start measurement"
                };
                painter.text(
                    mouse_pos + Vec2::new(20.0, -20.0),
                    egui::Align2::LEFT_BOTTOM,
                    text,
                    egui::FontId::default(),
                    Color32::CYAN,
                );
            }
        }
        
        // Handle professional ruler tool with right-click drag
        if app.ruler_active && !app.setting_origin_mode {
            handle_ruler_interaction(ui, app, response);
        }
        
        // Handle origin setting
        if app.setting_origin_mode && response.clicked() {
            if let Some(gerber_coords) = app.ui_state.cursor_gerber_coords {
                // Enterprise feature: Apply snap to grid if enabled
                let final_coords = if app.grid_settings.snap_enabled {
                    let point = nalgebra::Point2::new(gerber_coords.x, gerber_coords.y);
                    crate::display::snap_to_grid(point, &app.grid_settings)
                } else {
                    nalgebra::Point2::new(gerber_coords.x, gerber_coords.y)
                };
                
                app.display_manager.design_offset = crate::display::VectorOffset {
                    x: final_coords.x,
                    y: final_coords.y,
                };
                app.setting_origin_mode = false;
                app.origin_has_been_set = true;
                
                // Force view refresh to properly center coordinates at the new origin
                app.needs_initial_view = true;
                
                // Mark coordinates as dirty to force refresh
                crate::ecs::mark_coordinates_dirty_ecs(&mut app.ecs_world);
                
                let logger_state = app.logger_state.clone();
                let log_colors = app.log_colors.clone();
                let logger = ReactiveEventLogger::with_colors(&logger_state, &log_colors);
                let snap_msg = if app.grid_settings.snap_enabled { " (snapped to grid)" } else { "" };
                logger.log_info(&format!("Set origin to ({:.2}, {:.2}) mm{} - view recentered", final_coords.x, final_coords.y, snap_msg));
            }
        }
    }
}

fn handle_zoom_window(ui: &mut egui::Ui, app: &mut DemoLensApp, viewport: &Rect, mouse_pos_screen: Option<Pos2>, response: &egui::Response) {
    let right_button = egui::PointerButton::Secondary;
    
    // Start zoom window
    if response.contains_pointer() {
        if ui.input(|i| i.pointer.button_pressed(right_button)) {
            if let Some(pos) = mouse_pos_screen {
                app.zoom_window_start = Some(pos);
                app.zoom_window_dragging = true;
            }
        }
    }
    
    // Complete zoom window
    if app.zoom_window_dragging && ui.input(|i| i.pointer.button_released(right_button)) {
        if let (Some(start), Some(end)) = (app.zoom_window_start, ui.input(|i| i.pointer.hover_pos())) {
            let zoom_rect = Rect::from_two_pos(start, end);
            
            if zoom_rect.width() > 10.0 && zoom_rect.height() > 10.0 {
                let gerber_start = app.view_state.screen_to_gerber_coords(zoom_rect.min);
                let gerber_end = app.view_state.screen_to_gerber_coords(zoom_rect.max);
                
                let gerber_width = (gerber_end.x - gerber_start.x).abs() as f32;
                let gerber_height = (gerber_end.y - gerber_start.y).abs() as f32;
                
                let scale_x = viewport.width() / gerber_width;
                let scale_y = viewport.height() / gerber_height;
                let new_scale = scale_x.min(scale_y) * 0.9;
                
                let gerber_center_x = (gerber_start.x + gerber_end.x) / 2.0;
                let gerber_center_y = (gerber_start.y + gerber_end.y) / 2.0;
                
                app.view_state.scale = new_scale;
                
                let viewport_center = viewport.center();
                app.view_state.translation = Vec2::new(
                    viewport_center.x - (gerber_center_x * new_scale as f64) as f32,
                    viewport_center.y + (gerber_center_y * new_scale as f64) as f32
                );
                
                app.sync_zoom_to_ecs(); // Sync zoom changes to ECS
            }
        }
        
        app.zoom_window_dragging = false;
        app.zoom_window_start = None;
    }
    
    // Cancel zoom window on escape
    if app.zoom_window_dragging && ui.input(|i| i.key_pressed(egui::Key::Escape)) {
        app.zoom_window_dragging = false;
        app.zoom_window_start = None;
    }
}

fn handle_mouse_wheel_zoom(ui: &mut egui::Ui, app: &mut DemoLensApp, _viewport: &Rect, response: &egui::Response) {
    if !response.contains_pointer() {
        return;
    }
    
    let scroll_delta = ui.input(|i| i.raw_scroll_delta.y);
    if scroll_delta == 0.0 {
        return;
    }
    
    // Get mouse position for zoom center
    let mouse_pos = ui.input(|i| i.pointer.hover_pos());
    if let Some(mouse_pos) = mouse_pos {
        // Calculate zoom factor (positive scroll = zoom in, negative = zoom out)
        let zoom_factor = if scroll_delta > 0.0 {
            1.1 // Zoom in
        } else {
            1.0 / 1.1 // Zoom out
        };
        
        // Get current scale and calculate new scale
        let old_scale = app.view_state.scale;
        let new_scale = (old_scale * zoom_factor).clamp(0.01, 100.0);
        
        // Calculate the gerber coordinates at the mouse position before scaling
        let gerber_point = app.view_state.screen_to_gerber_coords(mouse_pos);
        
        // Update the scale
        app.view_state.scale = new_scale;
        
        // Calculate the new screen position of the same gerber point
        let new_screen_pos = app.view_state.gerber_to_screen_coords(gerber_point);
        
        // Adjust translation to keep the mouse cursor over the same gerber point
        let translation_adjustment = mouse_pos - new_screen_pos;
        app.view_state.translation += translation_adjustment;
        
        // Sync with ECS zoom resource
        app.sync_zoom_to_ecs();
    }
}

fn render_gerber_content(ui: &mut egui::Ui, app: &mut DemoLensApp, viewport: &Rect) {
    let painter = ui.painter_at(*viewport);
    painter.rect_filled(*viewport, 0.0, ui.visuals().extreme_bg_color);
    
    if app.needs_initial_view {
        app.reset_view(*viewport);
    }
    
    let painter = ui.painter().with_clip_rect(*viewport);
    
    // Draw grid
    crate::display::draw_grid(&painter, viewport, &app.view_state, &app.grid_settings);
    
    // Draw quadrant axes
    if app.display_manager.quadrant_view_enabled {
        draw_quadrant_axes(&painter, viewport, &app.view_state, app.ui_state.origin_screen_pos);
    }
    
    // Draw crosshairs - always at the active origin
    draw_crosshair(&painter, app.ui_state.origin_screen_pos, Color32::BLUE);
    
    // Render layers using ECS system (gerber-viewer 0.2.0 compatible)
    app.render_layers_ecs(&painter);
    
    // Render overlays
    render_overlays(app, &painter, viewport);
    
    // Render cursor info
    render_cursor_info(ui, app, &painter, viewport);
}


fn render_overlays(app: &mut DemoLensApp, painter: &Painter, viewport: &Rect) {
    let screen_radius = MARKER_RADIUS * app.view_state.scale;
    
    // Origin marker - show only the active origin point
    let design_offset = &app.display_manager.design_offset;
    let has_custom_origin = design_offset.x != 0.0 || design_offset.y != 0.0;
    
    if has_custom_origin {
        // Show custom origin (yellow marker) - this is the only visible origin
        let design_offset_screen_position = app.view_state.gerber_to_screen_coords(Vector2::from(design_offset.clone()).to_position().to_point2());
        draw_marker(painter, design_offset_screen_position, Color32::ORANGE, Color32::YELLOW, screen_radius);
    } else {
        // Show center origin (purple marker) when no custom origin is set
        let purple_dot_pos = if app.display_manager.quadrant_view_enabled {
            app.ui_state.center_screen_pos
        } else {
            app.ui_state.origin_screen_pos
        };
        draw_marker(painter, purple_dot_pos, Color32::PURPLE, Color32::MAGENTA, screen_radius);
    }
    
    // Corner overlay shapes
    render_corner_overlays(app, painter);
    
    // DRC violations
    render_drc_violations(app, painter);
    
    // Board dimensions
    render_board_dimensions(app, painter, viewport);
    
    // Enterprise feature: Ruler visualization
    render_ruler(app, painter);
    
    // Custom measurement crosshair
    render_measurement_crosshair(app, painter);
    
    // Zoom window
    render_zoom_window(app, painter);
}

fn render_corner_overlays(app: &mut DemoLensApp, painter: &Painter) {
    if !app.drc_manager.corner_overlay_shapes.is_empty() {
        let overlay_color = Color32::from_rgb(0, 255, 0);
        
        for shape in &app.drc_manager.corner_overlay_shapes {
            let mut transformed_vertices = Vec::new();
            
            for point in &shape.points {
                let mut vertex_pos = *point;
                
                // Apply rotation
                if app.rotation_degrees != 0.0 {
                    let rotation_radians = app.rotation_degrees.to_radians();
                    let (sin_theta, cos_theta) = (rotation_radians.sin(), rotation_radians.cos());
                    
                    let rotated_x = vertex_pos.x * cos_theta as f64 - vertex_pos.y * sin_theta as f64;
                    let rotated_y = vertex_pos.x * sin_theta as f64 + vertex_pos.y * cos_theta as f64;
                    vertex_pos = Position { x: rotated_x, y: rotated_y };
                }
                
                // Apply mirroring
                if app.display_manager.mirroring.x {
                    vertex_pos = vertex_pos.invert_x();
                }
                if app.display_manager.mirroring.y {
                    vertex_pos = vertex_pos.invert_y();
                }
                
                // Apply offsets
                let origin = Vector2::from(app.display_manager.center_offset.clone()) - Vector2::from(app.display_manager.design_offset.clone());
                vertex_pos = vertex_pos + origin.to_position();
                
                let vertex_screen = app.view_state.gerber_to_screen_coords(vertex_pos.to_point2());
                transformed_vertices.push(vertex_screen);
            }
            
            if transformed_vertices.len() >= 3 {
                painter.add(egui::Shape::convex_polygon(
                    transformed_vertices,
                    overlay_color,
                    Stroke::NONE
                ));
            }
        }
    }
}

fn render_drc_violations(app: &mut DemoLensApp, painter: &Painter) {
    for violation in &app.drc_manager.violations {
        let violation_pos = Position::new(violation.x as f64, violation.y as f64);
        let mut transformed_pos = violation_pos;
        
        // Apply rotation
        if app.rotation_degrees != 0.0 {
            let rotation_radians = app.rotation_degrees.to_radians();
            let (sin_theta, cos_theta) = (rotation_radians.sin(), rotation_radians.cos());
            let rotated_x = transformed_pos.x * cos_theta as f64 - transformed_pos.y * sin_theta as f64;
            let rotated_y = transformed_pos.x * sin_theta as f64 + transformed_pos.y * cos_theta as f64;
            transformed_pos = Position::new(rotated_x, rotated_y);
        }
        
        // Apply mirroring
        if app.display_manager.mirroring.x {
            transformed_pos = transformed_pos.invert_x();
        }
        if app.display_manager.mirroring.y {
            transformed_pos = transformed_pos.invert_y();
        }
        
        // Apply offsets
        let origin = Vector2::from(app.display_manager.center_offset.clone()) - Vector2::from(app.display_manager.design_offset.clone());
        transformed_pos = transformed_pos + origin.to_position();
        
        let screen_pos = app.view_state.gerber_to_screen_coords(transformed_pos.to_point2());
        
        let base_size = 3.0;
        let marker_size = base_size * app.view_state.scale.max(0.5);
        let color = Color32::RED;
        
        draw_violation_marker(painter, screen_pos, marker_size, color);
    }
}

fn render_board_dimensions(app: &mut DemoLensApp, painter: &Painter, viewport: &Rect) {
    if let Some((_entity, _layer_info, gerber_data, _visibility)) = crate::ecs::get_layer_data(&mut app.ecs_world, crate::ecs::LayerType::MechanicalOutline) {
        let bbox = gerber_data.0.bounding_box();
        let width_mm = bbox.width();
        let height_mm = bbox.height();
        
        let units_resource = Tab::get_units(app);
        let dimension_text = if units_resource.is_mils() {
            let width_nm = mm_to_nm(width_mm as f32);
            let height_nm = mm_to_nm(height_mm as f32);
            let width_mils = nm_to_mils(width_nm);
            let height_mils = nm_to_mils(height_nm);
            format!("{:.0} x {:.0} mils", width_mils, height_mils)
        } else {
            format!("{:.1} x {:.1} mm", width_mm, height_mm)
        };
        
        let text_pos = viewport.max - Vec2::new(10.0, 50.0);
        painter.text(
            text_pos,
            egui::Align2::RIGHT_BOTTOM,
            dimension_text,
            egui::FontId::default(),
            Color32::from_rgb(200, 200, 200),
        );
    }
}

fn render_zoom_window(app: &mut DemoLensApp, painter: &Painter) {
    if app.zoom_window_dragging {
        if let (Some(start), Some(current)) = (app.zoom_window_start, painter.ctx().input(|i| i.pointer.hover_pos())) {
            let zoom_rect = Rect::from_two_pos(start, current);
            
            // Draw semi-transparent fill
            painter.rect_filled(
                zoom_rect,
                0.0,
                Color32::from_rgba_unmultiplied(100, 150, 255, 50)
            );
            
            // Draw border
            let stroke = Stroke::new(2.0, Color32::from_rgb(100, 150, 255));
            painter.line_segment([zoom_rect.min, Pos2::new(zoom_rect.max.x, zoom_rect.min.y)], stroke);
            painter.line_segment([Pos2::new(zoom_rect.max.x, zoom_rect.min.y), zoom_rect.max], stroke);
            painter.line_segment([zoom_rect.max, Pos2::new(zoom_rect.min.x, zoom_rect.max.y)], stroke);
            painter.line_segment([Pos2::new(zoom_rect.min.x, zoom_rect.max.y), zoom_rect.min], stroke);
            
            // Draw corner markers
            let corner_size = 5.0;
            let corners = [zoom_rect.min, 
                          Pos2::new(zoom_rect.max.x, zoom_rect.min.y),
                          zoom_rect.max,
                          Pos2::new(zoom_rect.min.x, zoom_rect.max.y)];
            
            for corner in &corners {
                painter.circle_filled(*corner, corner_size, Color32::from_rgb(100, 150, 255));
            }
        }
    }
}

fn render_ruler(app: &mut DemoLensApp, painter: &Painter) {
    // Render active ruler if active
    if app.ruler_active {
        render_ruler_measurement(app, painter, app.ruler_start, app.ruler_end, true);
    }
    // Render latched ruler if not active but latched measurement exists
    else if app.latched_measurement_start.is_some() && app.latched_measurement_end.is_some() {
        render_ruler_measurement(app, painter, app.latched_measurement_start, app.latched_measurement_end, false);
    }
}

fn render_ruler_measurement(app: &mut DemoLensApp, painter: &Painter, start_opt: Option<nalgebra::Point2<f64>>, end_opt: Option<nalgebra::Point2<f64>>, is_active: bool) {
    // Draw ruler points and line
    if let Some(start) = start_opt {
        let start_screen = app.view_state.gerber_to_screen_coords(start);
        
        // Choose colors based on active/latched state
        let (point_color, line_color, text_color) = if is_active {
            (Color32::RED, Color32::WHITE, Color32::WHITE)
        } else {
            (Color32::GRAY, Color32::LIGHT_GRAY, Color32::LIGHT_GRAY)
        };
        
        // Draw start point
        painter.circle_filled(start_screen, 4.0, point_color);
        painter.circle_stroke(start_screen, 6.0, Stroke::new(2.0, line_color));
        
        if let Some(end) = end_opt {
            let end_screen = app.view_state.gerber_to_screen_coords(end);
            
            // Draw end point
            painter.circle_filled(end_screen, 4.0, point_color);
            painter.circle_stroke(end_screen, 6.0, Stroke::new(2.0, line_color));
            
            // Draw ruler line
            painter.line_segment(
                [start_screen, end_screen],
                Stroke::new(3.0, line_color)
            );
            
            let dx = end.x - start.x;
            let dy = end.y - start.y;
            let distance = (dx * dx + dy * dy).sqrt();
            
            // Create measurement text with dx/dy display
            let units_resource = Tab::get_units(app);
            let measurement_text = if units_resource.is_mils() {
                let distance_nm = mm_to_nm(distance as f32);
                let dx_nm = mm_to_nm(dx.abs() as f32);
                let dy_nm = mm_to_nm(dy.abs() as f32);
                format!(
                    "{:.2} mils\nΔX: {:.2}\nΔY: {:.2}",
                    nm_to_mils(distance_nm),
                    nm_to_mils(dx_nm),
                    nm_to_mils(dy_nm)
                )
            } else {
                format!(
                    "{:.3} mm\nΔX: {:.3}\nΔY: {:.3}",
                    distance,
                    dx,
                    dy
                )
            };
            
            // Position text near the end point (offset to avoid overlap)
            let text_offset = Vec2::new(20.0, -45.0);
            let text_pos = end_screen + text_offset;
            
            // Draw text background (only for active measurements)
            if is_active {
                let text_size = painter.text(
                    text_pos,
                    egui::Align2::LEFT_TOP,
                    "",
                    egui::FontId::monospace(16.0),
                    text_color,
                ).size();
                
                let background_rect = egui::Rect::from_min_size(
                    text_pos - Vec2::new(6.0, 6.0),
                    text_size + Vec2::new(12.0, 12.0)
                );
                painter.rect_filled(background_rect, 6.0, Color32::from_rgba_unmultiplied(0, 0, 0, 240));
            }
            
            // Draw measurement text at endpoint
            painter.text(
                text_pos,
                egui::Align2::LEFT_TOP,
                measurement_text,
                egui::FontId::monospace(16.0),
                text_color,
            );
        }
    }
}

fn handle_ruler_interaction(ui: &mut egui::Ui, app: &mut DemoLensApp, response: &egui::Response) {
    if !app.ruler_active {
        return;
    }
    
    let mouse_pos = ui.input(|i| i.pointer.hover_pos());
    
    // In ruler mode, left-click to set measurement points
    if response.clicked() {
        if let Some(mouse_screen_pos) = mouse_pos {
            let gerber_coords = app.view_state.screen_to_gerber_coords(mouse_screen_pos);
            
            // Apply snap to grid if enabled
            let final_coords = if app.grid_settings.snap_enabled {
                let point = nalgebra::Point2::new(gerber_coords.x, gerber_coords.y);
                crate::display::snap_to_grid(point, &app.grid_settings)
            } else {
                nalgebra::Point2::new(gerber_coords.x, gerber_coords.y)
            };
            
            if app.ruler_start.is_none() {
                // First click - set start point
                app.ruler_start = Some(final_coords);
                app.ruler_end = None;
                app.ruler_dragging = true; // Enable live preview
            } else if app.ruler_end.is_none() {
                // Second click - set end point and complete measurement
                app.ruler_end = Some(final_coords);
                app.ruler_dragging = false;
            } else {
                // Third click - start new measurement
                app.ruler_start = Some(final_coords);
                app.ruler_end = None;
                app.ruler_dragging = true;
            }
        }
    }
    
    // Show live preview when dragging (after first click, before second click)
    if app.ruler_dragging && app.ruler_start.is_some() && mouse_pos.is_some() {
        let mouse_screen_pos = mouse_pos.unwrap();
        let gerber_coords = app.view_state.screen_to_gerber_coords(mouse_screen_pos);
        
        // Apply snap to grid if enabled
        let final_coords = if app.grid_settings.snap_enabled {
            let point = nalgebra::Point2::new(gerber_coords.x, gerber_coords.y);
            crate::display::snap_to_grid(point, &app.grid_settings)
        } else {
            nalgebra::Point2::new(gerber_coords.x, gerber_coords.y)
        };
        
        // Update live preview end point
        app.ruler_end = Some(final_coords);
    }
}

fn render_cursor_info(ui: &mut egui::Ui, app: &mut DemoLensApp, painter: &Painter, viewport: &Rect) {
    // Hide cursor coordinates when ruler mode is active
    if app.ruler_active {
        return;
    }
    
    let mouse_pos_screen = ui.input(|i| i.pointer.hover_pos());
    
    if let Some(mouse_screen_pos) = mouse_pos_screen {
        if viewport.contains(mouse_screen_pos) {
            let gerber_pos = app.view_state.screen_to_gerber_coords(mouse_screen_pos);
            
            // Apply the design_offset as a simple coordinate offset for display
            // The design_offset is where we want (0,0) to be, so we subtract it from current position
            let adjusted_pos = Position::new(
                gerber_pos.x - app.display_manager.design_offset.x,
                gerber_pos.y - app.display_manager.design_offset.y
            );
            
            let units_resource = Tab::get_units(app);
            let cursor_text = if units_resource.is_mils() {
                let x_nm = mm_to_nm(adjusted_pos.x as f32);
                let y_nm = mm_to_nm(adjusted_pos.y as f32);
                let x_mils = nm_to_mils(x_nm);
                let y_mils = nm_to_mils(y_nm);
                format!("({:.0}, {:.0}) mils", x_mils, y_mils)
            } else {
                format!("({:.2}, {:.2}) mm", adjusted_pos.x, adjusted_pos.y)
            };
            
            
            let text_offset = Vec2::new(15.0, -15.0);
            let cursor_text_pos = mouse_screen_pos + text_offset;
            
            // Draw background
            let text_size = painter.text(
                cursor_text_pos,
                egui::Align2::LEFT_TOP,
                "",
                egui::FontId::monospace(12.0),
                Color32::WHITE,
            ).size();
            
            let background_rect = egui::Rect::from_min_size(
                cursor_text_pos - Vec2::new(2.0, 2.0),
                text_size + Vec2::new(4.0, 4.0)
            );
            
            painter.rect_filled(
                background_rect,
                3.0,
                Color32::from_rgba_unmultiplied(0, 0, 0, 180)
            );
            
            // Draw text
            painter.text(
                cursor_text_pos,
                egui::Align2::LEFT_TOP,
                cursor_text,
                egui::FontId::monospace(12.0),
                Color32::WHITE,
            );
            
            // Draw crosshair
            let crosshair_size = 8.0;
            painter.line_segment(
                [
                    mouse_screen_pos - Vec2::new(crosshair_size, 0.0),
                    mouse_screen_pos + Vec2::new(crosshair_size, 0.0)
                ],
                Stroke::new(1.0, Color32::WHITE)
            );
            painter.line_segment(
                [
                    mouse_screen_pos - Vec2::new(0.0, crosshair_size),
                    mouse_screen_pos + Vec2::new(0.0, crosshair_size)
                ],
                Stroke::new(1.0, Color32::WHITE)
            );
        }
    }
    
    // Unit display
    let unit_toggle_pos = viewport.max - Vec2::new(10.0, 30.0);
    let units_resource = Tab::get_units(app);
    let unit_text = if units_resource.is_mils() { "mils" } else { "mm" };
    painter.text(
        unit_toggle_pos,
        egui::Align2::RIGHT_BOTTOM,
        format!("Mouse: {}", unit_text),
        egui::FontId::default(),
        Color32::from_rgb(150, 150, 150),
    );
}

fn render_measurement_crosshair(app: &mut DemoLensApp, painter: &Painter) {
    // Skip if in origin setting mode
    if app.setting_origin_mode {
        return;
    }
    
    // Draw crosshairs for active measurement points
    if app.ruler_active {
        if let Some(start_point) = app.ruler_start {
            let screen_pos = app.view_state.gerber_to_screen_coords(start_point);
            draw_measurement_crosshair(painter, screen_pos, Color32::from_rgb(139, 0, 0)); // Dark red for active
        }
        
        if let Some(end_point) = app.ruler_end {
            let screen_pos = app.view_state.gerber_to_screen_coords(end_point);
            draw_measurement_crosshair(painter, screen_pos, Color32::from_rgb(139, 0, 0)); // Dark red for active
        }
    }
    // Draw crosshairs for latched measurement points (grayed out)
    else {
        if let Some(start_point) = app.latched_measurement_start {
            let screen_pos = app.view_state.gerber_to_screen_coords(start_point);
            draw_measurement_crosshair(painter, screen_pos, Color32::from_rgb(100, 100, 100)); // Gray for latched
        }
        
        if let Some(end_point) = app.latched_measurement_end {
            let screen_pos = app.view_state.gerber_to_screen_coords(end_point);
            draw_measurement_crosshair(painter, screen_pos, Color32::from_rgb(100, 100, 100)); // Gray for latched
        }
    }
}

fn draw_measurement_crosshair(painter: &Painter, center: Pos2, color: Color32) {
    let crosshair_size = 12.0;
    let stroke = egui::Stroke::new(2.0, color);
    
    // Horizontal line
    painter.line_segment(
        [
            center + Vec2::new(-crosshair_size, 0.0),
            center + Vec2::new(crosshair_size, 0.0),
        ],
        stroke,
    );
    
    // Vertical line
    painter.line_segment(
        [
            center + Vec2::new(0.0, -crosshair_size),
            center + Vec2::new(0.0, crosshair_size),
        ],
        stroke,
    );
}

pub struct TabViewer<'a> {
    pub app: &'a mut DemoLensApp,
}

impl<'a> egui_dock::TabViewer for TabViewer<'a> {
    type Tab = Tab;

    fn title(&mut self, tab: &mut Self::Tab) -> egui::WidgetText {
        tab.title().into()
    }

    fn ui(&mut self, ui: &mut egui::Ui, tab: &mut Self::Tab) {
        let mut params = TabParams {
            app: self.app,
        };
        tab.content(ui, &mut params);
    }
}

fn render_zoom_display(ui: &mut egui::Ui, app: &mut DemoLensApp) {
    // Get zoom info from ECS, fallback to legacy ViewState
    let (zoom_percentage, scale_factor) = if let Some(zoom_resource) = app.ecs_world.get_resource::<crate::ecs::ZoomResource>() {
        (zoom_resource.get_zoom_percentage(), zoom_resource.scale)
    } else {
        (app.view_state.scale * 100.0, app.view_state.scale)
    };
    
    // Format zoom display with appropriate precision
    let zoom_text = if zoom_percentage >= 100.0 {
        format!("🔍 {:.0}%", zoom_percentage)
    } else if zoom_percentage >= 10.0 {
        format!("🔍 {:.1}%", zoom_percentage)
    } else {
        format!("🔍 {:.2}%", zoom_percentage)
    };
    
    // Display zoom with a distinct visual style
    ui.label(egui::RichText::new(zoom_text)
        .color(egui::Color32::from_rgb(100, 200, 100))
        .strong())
        .on_hover_text(format!(
            "Current Zoom Level (ECS)\nScale Factor: {:.3}x\nPercentage: {:.2}%",
            scale_factor,
            zoom_percentage
        ));
}

/// Draw a red X marker for DRC violations
fn draw_violation_marker(painter: &Painter, center: Pos2, size: f32, color: Color32) {
    let half_size = size / 2.0;
    let stroke = Stroke::new(2.0, color);
    
    // Draw X shape
    painter.line_segment([
        Pos2::new(center.x - half_size, center.y - half_size),
        Pos2::new(center.x + half_size, center.y + half_size)
    ], stroke);
    
    painter.line_segment([
        Pos2::new(center.x - half_size, center.y + half_size),
        Pos2::new(center.x + half_size, center.y - half_size)
    ], stroke);
}

/// Draw quadrant axes when quadrant view is enabled
fn draw_quadrant_axes(painter: &Painter, viewport: &Rect, _view_state: &ViewState, center_screen_pos: Pos2) {
    let stroke = Stroke::new(2.0, Color32::from_rgba_unmultiplied(100, 100, 100, 150));
    
    // Draw vertical axis
    if center_screen_pos.x >= viewport.min.x && center_screen_pos.x <= viewport.max.x {
        painter.line_segment(
            [
                Pos2::new(center_screen_pos.x, viewport.min.y),
                Pos2::new(center_screen_pos.x, viewport.max.y)
            ],
            stroke
        );
    }
    
    // Draw horizontal axis
    if center_screen_pos.y >= viewport.min.y && center_screen_pos.y <= viewport.max.y {
        painter.line_segment(
            [
                Pos2::new(viewport.min.x, center_screen_pos.y),
                Pos2::new(viewport.max.x, center_screen_pos.y)
            ],
            stroke
        );
    }
    
    // Quadrant labels removed as requested by user
}