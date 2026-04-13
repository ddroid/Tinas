use gtk4::prelude::*;
use gtk4::{Box, Button, ColorButton, Label, Orientation, Scale, ScrolledWindow, Separator};
use std::cell::RefCell;
use std::rc::Rc;

use crate::editor::brush::{Brush, BrushShape, Eraser};
use crate::ui::editor_canvas::{EditorCanvas, Tool};

pub struct ToolPanel {
    container_full: ScrolledWindow,   // old wide panel (200px)
    container_mini: ScrolledWindow,   // new thin icon strip (48px)
    editor_canvas: Rc<RefCell<Option<Rc<EditorCanvas>>>>,
    current_color: Rc<RefCell<(f64, f64, f64)>>,
    current_size: Rc<RefCell<f64>>,
    current_shape: Rc<RefCell<BrushShape>>,
    is_brush_active: Rc<RefCell<bool>>,
}

impl ToolPanel {
    pub fn new() -> Self {
        let editor_canvas: Rc<RefCell<Option<Rc<EditorCanvas>>>> = Rc::new(RefCell::new(None));
        let current_color = Rc::new(RefCell::new((1.0_f64, 0.0_f64, 0.0_f64)));
        let current_size = Rc::new(RefCell::new(8.0_f64));
        let current_shape = Rc::new(RefCell::new(BrushShape::Circle));
        let is_brush_active = Rc::new(RefCell::new(true));

        let container_full = Self::build_full_ui(
            &editor_canvas, &current_color, &current_size, &current_shape, &is_brush_active,
        );
        let container_mini = Self::build_mini_ui(
            &editor_canvas, &current_color, &current_size, &current_shape, &is_brush_active,
        );

        Self {
            container_full,
            container_mini,
            editor_canvas,
            current_color,
            current_size,
            current_shape,
            is_brush_active,
        }
    }

    // ── Full (wide) panel — original UI ─────────────────────────────────────
    fn build_full_ui(
        editor_canvas: &Rc<RefCell<Option<Rc<EditorCanvas>>>>,
        current_color: &Rc<RefCell<(f64, f64, f64)>>,
        current_size: &Rc<RefCell<f64>>,
        current_shape: &Rc<RefCell<BrushShape>>,
        is_brush_active: &Rc<RefCell<bool>>,
    ) -> ScrolledWindow {
        let content = Box::new(Orientation::Vertical, 12);
        content.set_margin_top(12);
        content.set_margin_bottom(12);
        content.set_margin_start(12);
        content.set_margin_end(12);
        content.set_width_request(200);

        let container = ScrolledWindow::builder()
            .hscrollbar_policy(gtk4::PolicyType::Never)
            .vscrollbar_policy(gtk4::PolicyType::Automatic)
            .min_content_width(200)
            .max_content_width(216)
            .propagate_natural_width(true)
            .child(&content)
            .build();
        container.set_hexpand(false);

        // Tools Section
        let tools_label = Label::new(Some("Tools"));
        tools_label.add_css_class("heading");
        tools_label.set_halign(gtk4::Align::Start);
        content.append(&tools_label);

        let tool_box = Box::new(Orientation::Horizontal, 8);
        tool_box.set_halign(gtk4::Align::Center);

        let brush_btn = Button::with_label("Brush");
        brush_btn.add_css_class("suggested-action");
        let eraser_btn = Button::with_label("Eraser");

        Self::connect_tool_buttons(
            &brush_btn, &eraser_btn,
            editor_canvas, current_color, current_size, current_shape, is_brush_active,
        );

        tool_box.append(&brush_btn);
        tool_box.append(&eraser_btn);
        content.append(&tool_box);
        content.append(&Separator::new(Orientation::Horizontal));

        // Size Section
        let size_label = Label::new(Some("Size"));
        size_label.add_css_class("heading");
        size_label.set_halign(gtk4::Align::Start);
        content.append(&size_label);

        let size_scale = Scale::with_range(Orientation::Horizontal, 1.0, 50.0, 1.0);
        size_scale.set_value(8.0);
        size_scale.set_draw_value(true);
        size_scale.set_value_pos(gtk4::PositionType::Right);

        {
            let canvas_ref = editor_canvas.clone();
            let size_ref = current_size.clone();
            size_scale.connect_value_changed(move |scale| {
                let value = scale.value();
                *size_ref.borrow_mut() = value;
                if let Some(ref canvas) = *canvas_ref.borrow() {
                    canvas.set_brush_size(value);
                }
            });
        }
        content.append(&size_scale);
        content.append(&Separator::new(Orientation::Horizontal));

        // Brush Shape Section
        let shape_label = Label::new(Some("Brush Shape"));
        shape_label.add_css_class("heading");
        shape_label.set_halign(gtk4::Align::Start);
        content.append(&shape_label);

        let shape_box = Box::new(Orientation::Horizontal, 8);
        shape_box.set_halign(gtk4::Align::Center);

        let circle_btn = Button::with_label("Circle");
        let square_btn = Button::with_label("Square");
        let line_btn = Button::with_label("Line");
        circle_btn.add_css_class("suggested-action");

        Self::connect_shape_buttons(
            &circle_btn, &square_btn, &line_btn,
            editor_canvas, current_shape,
        );

        shape_box.append(&circle_btn);
        shape_box.append(&square_btn);
        shape_box.append(&line_btn);
        content.append(&shape_box);
        content.append(&Separator::new(Orientation::Horizontal));

        // Color Section
        let color_label = Label::new(Some("Color"));
        color_label.add_css_class("heading");
        color_label.set_halign(gtk4::Align::Start);
        content.append(&color_label);

        let color_btn = ColorButton::new();
        color_btn.set_rgba(&gtk4::gdk::RGBA::new(1.0, 0.0, 0.0, 1.0));
        color_btn.set_use_alpha(true);

        Self::connect_color_button(&color_btn, editor_canvas, current_color, current_shape, is_brush_active);

        let color_box = Box::new(Orientation::Horizontal, 8);
        color_box.append(&Label::new(Some("Brush Color:")));
        color_box.append(&color_btn);
        content.append(&color_box);
        content.append(&Separator::new(Orientation::Horizontal));

        // Preset Colors
        let presets_label = Label::new(Some("Presets"));
        presets_label.add_css_class("heading");
        presets_label.set_halign(gtk4::Align::Start);
        content.append(&presets_label);

        let presets_grid = Box::new(Orientation::Vertical, 4);
        let colors: &[(&str, (f64, f64, f64, f64))] = &[
            ("Black", (0.0, 0.0, 0.0, 1.0)),
            ("White", (1.0, 1.0, 1.0, 1.0)),
            ("Red",   (1.0, 0.0, 0.0, 1.0)),
            ("Green", (0.0, 1.0, 0.0, 1.0)),
            ("Blue",  (0.0, 0.0, 1.0, 1.0)),
            ("Yellow",(1.0, 1.0, 0.0, 1.0)),
        ];
        for &(name, (r, g, b, a)) in colors {
            let btn = Button::with_label(name);
            btn.set_has_frame(false);
            let provider = gtk4::CssProvider::new();
            let css = format!(
                "button {{ background-color: rgba({},{},{},{}); color: {}; }}",
                (r * 255.0) as u8, (g * 255.0) as u8, (b * 255.0) as u8, a,
                if (r + g + b) / 3.0 > 0.5 { "black" } else { "white" }
            );
            provider.load_from_data(&css);
            btn.style_context().add_provider(&provider, gtk4::STYLE_PROVIDER_PRIORITY_APPLICATION);

            let canvas_ref = editor_canvas.clone();
            let color_ref = current_color.clone();
            let is_brush = is_brush_active.clone();
            let shape_ref = current_shape.clone();
            let color_btn_weak = color_btn.downgrade();
            btn.connect_clicked(move |_| {
                *color_ref.borrow_mut() = (r, g, b);
                if let Some(ref cb) = color_btn_weak.upgrade() {
                    cb.set_rgba(&gtk4::gdk::RGBA::new(r as f32, g as f32, b as f32, a as f32));
                }
                if *is_brush.borrow() {
                    if let Some(ref canvas) = *canvas_ref.borrow() {
                        canvas.set_brush_color(r, g, b, a);
                        canvas.set_brush_shape(*shape_ref.borrow());
                    }
                }
            });
            presets_grid.append(&btn);
        }
        content.append(&presets_grid);
        content.append(&Separator::new(Orientation::Horizontal));

        // Actions
        let actions_label = Label::new(Some("Actions"));
        actions_label.add_css_class("heading");
        actions_label.set_halign(gtk4::Align::Start);
        content.append(&actions_label);

        let clear_btn = Button::with_label("Clear Drawing");
        clear_btn.add_css_class("destructive-action");
        {
            let canvas_ref = editor_canvas.clone();
            clear_btn.connect_clicked(move |_| {
                if let Some(ref canvas) = *canvas_ref.borrow() {
                    canvas.clear_overlay();
                }
            });
        }
        content.append(&clear_btn);

        container
    }

    // ── Mini (thin) panel — icon strip ──────────────────────────────────────
    fn build_mini_ui(
        editor_canvas: &Rc<RefCell<Option<Rc<EditorCanvas>>>>,
        current_color: &Rc<RefCell<(f64, f64, f64)>>,
        current_size: &Rc<RefCell<f64>>,
        current_shape: &Rc<RefCell<BrushShape>>,
        is_brush_active: &Rc<RefCell<bool>>,
    ) -> ScrolledWindow {
        let content = Box::new(Orientation::Vertical, 8);
        content.set_margin_top(8);
        content.set_margin_bottom(8);
        content.set_margin_start(4);
        content.set_margin_end(4);
        content.set_width_request(48);

        let container = ScrolledWindow::builder()
            .hscrollbar_policy(gtk4::PolicyType::Never)
            .vscrollbar_policy(gtk4::PolicyType::Automatic)
            .min_content_width(48)
            .max_content_width(56)
            .propagate_natural_width(true)
            .child(&content)
            .build();
        container.set_hexpand(false);

        // Tools
        let brush_btn = Button::from_icon_name("document-edit-symbolic");
        brush_btn.set_tooltip_text(Some("Brush"));
        brush_btn.add_css_class("suggested-action");
        let eraser_btn = Button::from_icon_name("edit-clear-symbolic");
        eraser_btn.set_tooltip_text(Some("Eraser"));

        Self::connect_tool_buttons(
            &brush_btn, &eraser_btn,
            editor_canvas, current_color, current_size, current_shape, is_brush_active,
        );
        content.append(&brush_btn);
        content.append(&eraser_btn);
        content.append(&Separator::new(Orientation::Horizontal));

        // Shapes
        let circle_btn = Button::from_icon_name("media-record-symbolic");
        circle_btn.set_tooltip_text(Some("Circle"));
        circle_btn.add_css_class("suggested-action");
        let square_btn = Button::from_icon_name("media-playback-stop-symbolic");
        square_btn.set_tooltip_text(Some("Square"));
        let line_btn = Button::from_icon_name("list-remove-symbolic");
        line_btn.set_tooltip_text(Some("Line"));

        Self::connect_shape_buttons(
            &circle_btn, &square_btn, &line_btn,
            editor_canvas, current_shape,
        );
        content.append(&circle_btn);
        content.append(&square_btn);
        content.append(&line_btn);
        content.append(&Separator::new(Orientation::Horizontal));

        // Color
        let color_btn = ColorButton::new();
        color_btn.set_rgba(&gtk4::gdk::RGBA::new(1.0, 0.0, 0.0, 1.0));
        color_btn.set_use_alpha(true);
        color_btn.set_tooltip_text(Some("Brush Color"));
        Self::connect_color_button(&color_btn, editor_canvas, current_color, current_shape, is_brush_active);
        content.append(&color_btn);
        content.append(&Separator::new(Orientation::Horizontal));

        // Size slider (vertical)
        let size_scale = Scale::with_range(Orientation::Vertical, 1.0, 50.0, 1.0);
        size_scale.set_value(8.0);
        size_scale.set_draw_value(false);
        size_scale.set_inverted(true);
        size_scale.set_vexpand(true);
        size_scale.set_tooltip_text(Some("Brush Size"));
        {
            let canvas_ref = editor_canvas.clone();
            let size_ref = current_size.clone();
            size_scale.connect_value_changed(move |scale| {
                let value = scale.value();
                *size_ref.borrow_mut() = value;
                if let Some(ref canvas) = *canvas_ref.borrow() {
                    canvas.set_brush_size(value);
                }
            });
        }
        content.append(&size_scale);
        content.append(&Separator::new(Orientation::Horizontal));

        // Clear
        let clear_btn = Button::from_icon_name("edit-delete-symbolic");
        clear_btn.set_tooltip_text(Some("Clear Drawing"));
        clear_btn.add_css_class("destructive-action");
        {
            let canvas_ref = editor_canvas.clone();
            clear_btn.connect_clicked(move |_| {
                if let Some(ref canvas) = *canvas_ref.borrow() {
                    canvas.clear_overlay();
                }
            });
        }
        content.append(&clear_btn);

        container
    }

    // ── Shared signal helpers ────────────────────────────────────────────────

    fn connect_tool_buttons(
        brush_btn: &Button,
        eraser_btn: &Button,
        editor_canvas: &Rc<RefCell<Option<Rc<EditorCanvas>>>>,
        current_color: &Rc<RefCell<(f64, f64, f64)>>,
        current_size: &Rc<RefCell<f64>>,
        current_shape: &Rc<RefCell<BrushShape>>,
        is_brush_active: &Rc<RefCell<bool>>,
    ) {
        let canvas_ref = editor_canvas.clone();
        let is_brush = is_brush_active.clone();
        let color_ref = current_color.clone();
        let size_ref = current_size.clone();
        let shape_ref = current_shape.clone();
        let brush_weak = brush_btn.downgrade();
        let eraser_weak = eraser_btn.downgrade();
        brush_btn.connect_clicked(move |_| {
            *is_brush.borrow_mut() = true;
            if let Some(ref canvas) = *canvas_ref.borrow() {
                let (r, g, b) = *color_ref.borrow();
                let size = *size_ref.borrow();
                let shape = *shape_ref.borrow();
                canvas.set_tool(Tool::Brush(Brush::new(size, (r, g, b, 1.0)).with_shape(shape)));
            }
            if let Some(b) = brush_weak.upgrade()  { b.add_css_class("suggested-action"); }
            if let Some(b) = eraser_weak.upgrade() { b.remove_css_class("suggested-action"); }
        });

        let canvas_ref2 = editor_canvas.clone();
        let is_brush2 = is_brush_active.clone();
        let size_ref2 = current_size.clone();
        let shape_ref2 = current_shape.clone();
        let brush_weak2 = brush_btn.downgrade();
        let eraser_weak2 = eraser_btn.downgrade();
        eraser_btn.connect_clicked(move |_| {
            *is_brush2.borrow_mut() = false;
            if let Some(ref canvas) = *canvas_ref2.borrow() {
                let size = *size_ref2.borrow();
                let mut eraser = Eraser::new(size);
                eraser.shape = *shape_ref2.borrow();
                canvas.set_tool(Tool::Eraser(eraser));
            }
            if let Some(b) = brush_weak2.upgrade()  { b.remove_css_class("suggested-action"); }
            if let Some(b) = eraser_weak2.upgrade() { b.add_css_class("suggested-action"); }
        });
    }

    fn connect_shape_buttons(
        circle_btn: &Button,
        square_btn: &Button,
        line_btn: &Button,
        editor_canvas: &Rc<RefCell<Option<Rc<EditorCanvas>>>>,
        current_shape: &Rc<RefCell<BrushShape>>,
    ) {
        for (btn, shape) in [
            (circle_btn, BrushShape::Circle),
            (square_btn, BrushShape::Square),
            (line_btn,   BrushShape::Line),
        ] {
            let canvas_ref = editor_canvas.clone();
            let shape_ref = current_shape.clone();
            let circle_w = circle_btn.downgrade();
            let square_w = square_btn.downgrade();
            let line_w   = line_btn.downgrade();
            btn.connect_clicked(move |_| {
                *shape_ref.borrow_mut() = shape;
                if let Some(ref canvas) = *canvas_ref.borrow() {
                    canvas.set_brush_shape(shape);
                }
                if let Some(b) = circle_w.upgrade() {
                    if shape == BrushShape::Circle { b.add_css_class("suggested-action"); }
                    else { b.remove_css_class("suggested-action"); }
                }
                if let Some(b) = square_w.upgrade() {
                    if shape == BrushShape::Square { b.add_css_class("suggested-action"); }
                    else { b.remove_css_class("suggested-action"); }
                }
                if let Some(b) = line_w.upgrade() {
                    if shape == BrushShape::Line { b.add_css_class("suggested-action"); }
                    else { b.remove_css_class("suggested-action"); }
                }
            });
        }
    }

    fn connect_color_button(
        color_btn: &ColorButton,
        editor_canvas: &Rc<RefCell<Option<Rc<EditorCanvas>>>>,
        current_color: &Rc<RefCell<(f64, f64, f64)>>,
        current_shape: &Rc<RefCell<BrushShape>>,
        is_brush_active: &Rc<RefCell<bool>>,
    ) {
        let canvas_ref = editor_canvas.clone();
        let color_ref = current_color.clone();
        let is_brush = is_brush_active.clone();
        let shape_ref = current_shape.clone();
        color_btn.connect_color_set(move |btn| {
            let rgba = btn.rgba();
            let (r, g, b, a) = (rgba.red() as f64, rgba.green() as f64, rgba.blue() as f64, rgba.alpha() as f64);
            *color_ref.borrow_mut() = (r, g, b);
            if *is_brush.borrow() {
                if let Some(ref canvas) = *canvas_ref.borrow() {
                    canvas.set_brush_color(r, g, b, a);
                    canvas.set_brush_shape(*shape_ref.borrow());
                }
            }
        });
    }

    // ── Public API ───────────────────────────────────────────────────────────

    pub fn set_editor_canvas(&self, canvas: Rc<EditorCanvas>) {
        *self.editor_canvas.borrow_mut() = Some(Rc::clone(&canvas));
        let (r, g, b) = *self.current_color.borrow();
        let size  = *self.current_size.borrow();
        let shape = *self.current_shape.borrow();
        if *self.is_brush_active.borrow() {
            canvas.set_tool(Tool::Brush(Brush::new(size, (r, g, b, 1.0)).with_shape(shape)));
        } else {
            let mut eraser = Eraser::new(size);
            eraser.shape = shape;
            canvas.set_tool(Tool::Eraser(eraser));
        }
    }

    /// The full-width (old) panel widget.
    pub fn widget_full(&self) -> &ScrolledWindow { &self.container_full }

    /// The slim icon-strip widget.
    pub fn widget_mini(&self) -> &ScrolledWindow { &self.container_mini }
}

impl Default for ToolPanel {
    fn default() -> Self { Self::new() }
}
