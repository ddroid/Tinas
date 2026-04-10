use gtk4::prelude::*;
use gtk4::{Box, Button, ColorButton, Label, Orientation, Scale, ScrolledWindow, Separator};
use std::cell::RefCell;
use std::rc::Rc;

use crate::editor::brush::{Brush, Eraser};
use crate::ui::editor_canvas::{EditorCanvas, Tool};

pub struct ToolPanel {
    container: ScrolledWindow,
    content: Box,
    editor_canvas: RefCell<Option<Rc<EditorCanvas>>>,
    current_color: RefCell<(f64, f64, f64, f64)>,
    current_size: RefCell<f64>,
    is_brush_active: RefCell<bool>,
}

impl ToolPanel {
    pub fn new() -> Self {
        let content = Box::new(Orientation::Vertical, 12);
        content.set_margin_top(12);
        content.set_margin_bottom(12);
        content.set_margin_start(12);
        content.set_margin_end(12);
        content.set_width_request(200);
        
        let container = ScrolledWindow::builder()
            .hscrollbar_policy(gtk4::PolicyType::Never)
            .vscrollbar_policy(gtk4::PolicyType::Automatic)
            .child(&content)
            .build();
        
        let this = Self {
            container,
            content,
            editor_canvas: RefCell::new(None),
            current_color: RefCell::new((0.0, 0.0, 0.0, 1.0)),
            current_size: RefCell::new(8.0),
            is_brush_active: RefCell::new(true),
        };
        
        this.build_ui();
        this
    }
    
    fn build_ui(&self) {
        // Tools Section
        let tools_label = Label::new(Some("Tools"));
        tools_label.add_css_class("heading");
        tools_label.set_halign(gtk4::Align::Start);
        self.content.append(&tools_label);
        
        // Tool buttons row
        let tool_box = Box::new(Orientation::Horizontal, 8);
        tool_box.set_halign(gtk4::Align::Center);
        
        let brush_btn = Button::with_label("Brush");
        brush_btn.add_css_class("suggested-action");
        let eraser_btn = Button::with_label("Eraser");
        
        // Clone references for callbacks
        let canvas_ref = self.editor_canvas.clone();
        let is_brush = self.is_brush_active.clone();
        let color_ref = self.current_color.clone();
        let size_ref = self.current_size.clone();
        let brush_btn_weak = brush_btn.downgrade();
        let eraser_btn_weak = eraser_btn.downgrade();
        
        brush_btn.connect_clicked(move |_btn| {
            *is_brush.borrow_mut() = true;
            if let Some(ref canvas) = *canvas_ref.borrow() {
                let (r, g, b, a) = *color_ref.borrow();
                let size = *size_ref.borrow();
                canvas.set_tool(Tool::Brush(Brush::new(size, (r, g, b, a))));
            }
            if let Some(ref btn) = brush_btn_weak.upgrade() {
                btn.add_css_class("suggested-action");
            }
            if let Some(ref btn) = eraser_btn_weak.upgrade() {
                btn.remove_css_class("suggested-action");
            }
        });
        
        let canvas_ref2 = self.editor_canvas.clone();
        let is_brush2 = self.is_brush_active.clone();
        let size_ref2 = self.current_size.clone();
        let brush_btn_weak2 = brush_btn.downgrade();
        let eraser_btn_weak2 = eraser_btn.downgrade();
        
        eraser_btn.connect_clicked(move |_btn| {
            *is_brush2.borrow_mut() = false;
            if let Some(ref canvas) = *canvas_ref2.borrow() {
                let size = *size_ref2.borrow();
                canvas.set_tool(Tool::Eraser(Eraser::new(size)));
            }
            if let Some(ref btn) = brush_btn_weak2.upgrade() {
                btn.remove_css_class("suggested-action");
            }
            if let Some(ref btn) = eraser_btn_weak2.upgrade() {
                btn.add_css_class("suggested-action");
            }
        });
        
        tool_box.append(&brush_btn);
        tool_box.append(&eraser_btn);
        self.content.append(&tool_box);
        
        self.content.append(&Separator::new(Orientation::Horizontal));
        
        // Brush Size Section
        let size_label = Label::new(Some("Size"));
        size_label.add_css_class("heading");
        size_label.set_halign(gtk4::Align::Start);
        self.content.append(&size_label);
        
        let size_scale = Scale::with_range(Orientation::Horizontal, 1.0, 50.0, 1.0);
        size_scale.set_value(8.0);
        size_scale.set_draw_value(true);
        size_scale.set_value_pos(gtk4::PositionType::Right);
        
        let canvas_ref = self.editor_canvas.clone();
        let size_ref = self.current_size.clone();
        size_scale.connect_value_changed(move |scale| {
            let value = scale.value();
            *size_ref.borrow_mut() = value;
            if let Some(ref canvas) = *canvas_ref.borrow() {
                canvas.set_brush_size(value);
            }
        });
        
        self.content.append(&size_scale);
        
        self.content.append(&Separator::new(Orientation::Horizontal));
        
        // Color Section
        let color_label = Label::new(Some("Color"));
        color_label.add_css_class("heading");
        color_label.set_halign(gtk4::Align::Start);
        self.content.append(&color_label);
        
        let color_btn = ColorButton::new();
        color_btn.set_rgba(&gtk4::gdk::RGBA::new(0.0, 0.0, 0.0, 1.0));
        color_btn.set_use_alpha(true);
        
        let canvas_ref = self.editor_canvas.clone();
        let color_ref = self.current_color.clone();
        let is_brush = self.is_brush_active.clone();
        let size_ref = self.current_size.clone();
        
        color_btn.connect_color_set(move |btn| {
            let rgba = btn.rgba();
            let (r, g, b, a) = (rgba.red() as f64, rgba.green() as f64, rgba.blue() as f64, rgba.alpha() as f64);
            *color_ref.borrow_mut() = (r, g, b, a);
            
            if *is_brush.borrow() {
                if let Some(ref canvas) = *canvas_ref.borrow() {
                    let size = *size_ref.borrow();
                    canvas.set_brush_color(r, g, b, a);
                    canvas.set_tool(Tool::Brush(Brush::new(size, (r, g, b, a))));
                }
            }
        });
        
        let color_box = Box::new(Orientation::Horizontal, 8);
        color_box.append(&Label::new(Some("Brush Color:")));
        color_box.append(&color_btn);
        self.content.append(&color_box);
        
        self.content.append(&Separator::new(Orientation::Horizontal));
        
        // Preset Colors
        let presets_label = Label::new(Some("Presets"));
        presets_label.add_css_class("heading");
        presets_label.set_halign(gtk4::Align::Start);
        self.content.append(&presets_label);
        
        let presets_grid = Box::new(Orientation::Vertical, 4);
        
        let colors = vec![
            ("Black", (0.0, 0.0, 0.0, 1.0)),
            ("White", (1.0, 1.0, 1.0, 1.0)),
            ("Red", (1.0, 0.0, 0.0, 1.0)),
            ("Green", (0.0, 1.0, 0.0, 1.0)),
            ("Blue", (0.0, 0.0, 1.0, 1.0)),
            ("Yellow", (1.0, 1.0, 0.0, 1.0)),
        ];
        
        for (name, (r, g, b, a)) in colors {
            let btn = Button::with_label(name);
            btn.set_has_frame(false);
            
            // Set button color via CSS
            let provider = gtk4::CssProvider::new();
            let css = format!(
                "button {{ background-color: rgba({}, {}, {}, {}); color: {}; }}",
                (r * 255.0) as u8,
                (g * 255.0) as u8,
                (b * 255.0) as u8,
                a,
                if (r + g + b) / 3.0 > 0.5 { "black" } else { "white" }
            );
            provider.load_from_data(&css);
            btn.style_context().add_provider(&provider, gtk4::STYLE_PROVIDER_PRIORITY_APPLICATION);
            
            let canvas_ref = self.editor_canvas.clone();
            let color_ref = self.current_color.clone();
            let is_brush = self.is_brush_active.clone();
            let size_ref = self.current_size.clone();
            let color_btn_weak = color_btn.downgrade();
            
            btn.connect_clicked(move |_btn| {
                *color_ref.borrow_mut() = (r, g, b, a);
                if let Some(ref btn) = color_btn_weak.upgrade() {
                    btn.set_rgba(&gtk4::gdk::RGBA::new(r as f32, g as f32, b as f32, a as f32));
                }
                if *is_brush.borrow() {
                    if let Some(ref canvas) = *canvas_ref.borrow() {
                        let size = *size_ref.borrow();
                        canvas.set_brush_color(r, g, b, a);
                        canvas.set_tool(Tool::Brush(Brush::new(size, (r, g, b, a))));
                    }
                }
            });
            
            presets_grid.append(&btn);
        }
        
        self.content.append(&presets_grid);
        
        self.content.append(&Separator::new(Orientation::Horizontal));
        
        // Actions Section
        let actions_label = Label::new(Some("Actions"));
        actions_label.add_css_class("heading");
        actions_label.set_halign(gtk4::Align::Start);
        self.content.append(&actions_label);
        
        let clear_btn = Button::with_label("Clear Drawing");
        clear_btn.add_css_class("destructive-action");
        
        let canvas_ref = self.editor_canvas.clone();
        clear_btn.connect_clicked(move |_btn| {
            if let Some(ref canvas) = *canvas_ref.borrow() {
                canvas.clear_overlay();
            }
        });
        
        self.content.append(&clear_btn);
    }
    
    pub fn set_editor_canvas(&self, canvas: Rc<EditorCanvas>) {
        *self.editor_canvas.borrow_mut() = Some(canvas);
    }
    
    pub fn widget(&self) -> &ScrolledWindow {
        &self.container
    }
}

impl Default for ToolPanel {
    fn default() -> Self {
        Self::new()
    }
}
