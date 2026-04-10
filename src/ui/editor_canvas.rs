use gtk4::prelude::*;
use gtk4::{DrawingArea, EventControllerMotion, GestureDrag, GestureClick, Box, Orientation, ScrolledWindow};
use cairo::ImageSurface;
use std::cell::RefCell;
use std::rc::Rc;

use crate::editor::brush::{Brush, BrushShape, Eraser};
use crate::editor::history::History;

pub enum Tool {
    Brush(Brush),
    Eraser(Eraser),
}

pub struct EditorCanvas {
    container: ScrolledWindow,
    drawing_area: DrawingArea,
    base_surface: Rc<RefCell<Option<ImageSurface>>>,
    overlay_surface: Rc<RefCell<Option<ImageSurface>>>,
    current_tool: Rc<RefCell<Tool>>,
    history: Rc<RefCell<History>>,
    last_pos: RefCell<Option<(f64, f64)>>,
    hover_pos: Rc<RefCell<Option<(f64, f64)>>>,
    drag_happened: Rc<RefCell<bool>>,
    original_dimensions: RefCell<Option<(u32, u32)>>,
    has_changes: RefCell<bool>,
    on_changed: Rc<RefCell<Option<std::boxed::Box<dyn Fn(bool)>>>>,
}

impl EditorCanvas {
    pub fn new() -> Self {
        let drawing_area = DrawingArea::builder()
            .width_request(400)
            .height_request(400)
            .can_focus(true)
            .focusable(true)
            .build();
        
        // Enable all input events
        drawing_area.set_focusable(true);
        drawing_area.set_can_target(true);
        
        let container = Box::new(Orientation::Vertical, 0);
        container.append(&drawing_area);
        
        // Add scroll window for large images
        let scrolled = gtk4::ScrolledWindow::builder()
            .hscrollbar_policy(gtk4::PolicyType::Automatic)
            .vscrollbar_policy(gtk4::PolicyType::Automatic)
            .child(&container)
            .build();
        
        let this = Self {
            container: scrolled,
            drawing_area,
            base_surface: Rc::new(RefCell::new(None)),
            overlay_surface: Rc::new(RefCell::new(None)),
            current_tool: Rc::new(RefCell::new(Tool::Brush(Brush::new(8.0, (1.0, 0.0, 0.0, 1.0))))),
            history: Rc::new(RefCell::new(History::new(50))),
            last_pos: RefCell::new(None),
            hover_pos: Rc::new(RefCell::new(None)),
            drag_happened: Rc::new(RefCell::new(false)),
            original_dimensions: RefCell::new(None),
            has_changes: RefCell::new(false),
            on_changed: Rc::new(RefCell::new(None)),
        };
        
        this.setup_drawing();
        this.setup_input();
        
        this
    }
    
    fn setup_drawing(&self) {
        let base_surface = self.base_surface.clone();
        let overlay_surface = self.overlay_surface.clone();
        let hover_pos = self.hover_pos.clone();
        let current_tool = self.current_tool.clone();
        
        self.drawing_area.set_draw_func(move |_area, ctx, _width, _height| {
            // Clear background
            ctx.set_source_rgb(0.2, 0.2, 0.2);
            ctx.paint().expect("Failed to clear canvas");
            
            // Draw base surface (original image)
            if let Some(ref surface) = *base_surface.borrow() {
                ctx.set_source_surface(surface, 0.0, 0.0).ok();
                ctx.paint().ok();
            }
            
            // Draw overlay surface (user edits)
            if let Some(ref surface) = *overlay_surface.borrow() {
                ctx.set_source_surface(surface, 0.0, 0.0).ok();
                ctx.paint().ok();
            }

            if let Some((x, y)) = *hover_pos.borrow() {
                match &*current_tool.borrow() {
                    Tool::Brush(brush) => draw_tool_preview(ctx, x, y, brush.size, brush.shape, Some(brush.color)),
                    Tool::Eraser(eraser) => draw_tool_preview(ctx, x, y, eraser.size, eraser.shape, None),
                }
            }
        });
    }
    
    fn setup_input(&self) {
        let current_tool = Rc::clone(&self.current_tool);
        let overlay_surface = self.overlay_surface.clone();
        let last_pos = self.last_pos.clone();
        let history = Rc::clone(&self.history);
        let drawing_area = self.drawing_area.clone();
        let has_changes = self.has_changes.clone();
        let on_changed = self.on_changed.clone();
        let hover_pos = self.hover_pos.clone();
        let drag_happened = self.drag_happened.clone();
        
        // Clone for second closure
        let overlay_surface_clone = overlay_surface.clone();
        let last_pos_clone = last_pos.clone();
        let history_clone = Rc::clone(&history);
        let current_tool_clone = Rc::clone(&current_tool);
        let has_changes_clone = has_changes.clone();
        let on_changed_clone = on_changed.clone();
        let drawing_area_clone = drawing_area.clone();
        let hover_pos_clone = hover_pos.clone();
        let drag_happened_clone = drag_happened.clone();
        
        // Clone for click handler
        let overlay_surface_click = overlay_surface.clone();
        let current_tool_click = Rc::clone(&current_tool);
        let history_click = Rc::clone(&history);
        let has_changes_click = has_changes.clone();
        let on_changed_click = on_changed.clone();
        let drawing_area_click = drawing_area.clone();
        let drag_happened_click = drag_happened.clone();
        let hover_pos_click = hover_pos.clone();
        let last_pos_click = self.last_pos.clone();
        let hover_pos_drag_end = hover_pos.clone();

        let motion = EventControllerMotion::new();
        let hover_pos_motion = hover_pos.clone();
        let drawing_area_motion = drawing_area.clone();
        motion.connect_motion(move |_controller, x, y| {
            *hover_pos_motion.borrow_mut() = Some((x, y));
            drawing_area_motion.queue_draw();
        });

        let hover_pos_leave = hover_pos.clone();
        let drawing_area_leave = drawing_area.clone();
        motion.connect_leave(move |_controller| {
            *hover_pos_leave.borrow_mut() = None;
            drawing_area_leave.queue_draw();
        });

        self.drawing_area.add_controller(motion);
        
        // Drag gesture for drawing
        let drag = GestureDrag::new();
        drag.set_button(gtk4::gdk::BUTTON_PRIMARY);
        
        drag.connect_drag_begin(move |_gesture, x, y| {
            *drag_happened.borrow_mut() = false;
            *last_pos.borrow_mut() = None;
            *hover_pos.borrow_mut() = Some((x, y));

            drawing_area.queue_draw();
        });
        
        drag.connect_drag_update(move |_gesture, offset_x, offset_y| {
            if let Some((start_x, start_y)) = _gesture.start_point() {
                let current_x = start_x + offset_x;
                let current_y = start_y + offset_y;
                *hover_pos_clone.borrow_mut() = Some((current_x, current_y));

                let drag_distance = ((current_x - start_x).powi(2) + (current_y - start_y).powi(2)).sqrt();
                let stroke_started = *drag_happened_clone.borrow();

                if !stroke_started && drag_distance < 3.0 {
                    drawing_area_clone.queue_draw();
                    return;
                }

                if let Some(ref mut surface) = *overlay_surface_clone.borrow_mut() {
                    if !stroke_started {
                        history_clone.borrow_mut().push(surface);
                    }

                    let ctx = cairo::Context::new(surface).expect("Failed to create context");

                    if !stroke_started {
                        match &*current_tool_clone.borrow() {
                            Tool::Brush(brush) => {
                                brush.draw(&ctx, start_x, start_y);
                                brush.draw_line(&ctx, start_x, start_y, current_x, current_y);
                            }
                            Tool::Eraser(eraser) => {
                                eraser.erase(&ctx, start_x, start_y);
                                eraser.erase_line(&ctx, start_x, start_y, current_x, current_y);
                            }
                        }

                        *drag_happened_clone.borrow_mut() = true;
                    } else if let Some((last_x, last_y)) = *last_pos_clone.borrow() {
                        if (current_x - last_x).abs() > 0.1 || (current_y - last_y).abs() > 0.1 {
                            match &*current_tool_clone.borrow() {
                                Tool::Brush(brush) => {
                                    brush.draw_line(&ctx, last_x, last_y, current_x, current_y);
                                }
                                Tool::Eraser(eraser) => {
                                    eraser.erase_line(&ctx, last_x, last_y, current_x, current_y);
                                }
                            }
                        }
                    }

                    *last_pos_clone.borrow_mut() = Some((current_x, current_y));

                    if !*has_changes_clone.borrow() {
                        *has_changes_clone.borrow_mut() = true;
                        if let Some(ref callback) = *on_changed_clone.borrow() {
                            callback(true);
                        }
                    }

                    drawing_area_clone.queue_draw();
                }
            }
        });

        let last_pos_end = self.last_pos.clone();
        let drawing_area_end = self.drawing_area.clone();
        drag.connect_drag_end(move |_gesture, offset_x, offset_y| {
            if let Some((start_x, start_y)) = _gesture.start_point() {
                let current_x = start_x + offset_x;
                let current_y = start_y + offset_y;
                *last_pos_end.borrow_mut() = None;
                *hover_pos_drag_end.borrow_mut() = Some((current_x, current_y));
            } else {
                *last_pos_end.borrow_mut() = None;
            }
            drawing_area_end.queue_draw();
        });
        
        self.drawing_area.add_controller(drag);
        
        // Click gesture for single clicks
        let click = GestureClick::new();
        click.set_button(gtk4::gdk::BUTTON_PRIMARY);
        
        click.connect_released(move |_gesture, _n_press, x, y| {
            let was_drag = *drag_happened_click.borrow();
            *drag_happened_click.borrow_mut() = false;
            *last_pos_click.borrow_mut() = None;

            if was_drag {
                return;
            }

            // Save state before drawing
            if let Some(ref mut surface) = *overlay_surface_click.borrow_mut() {
                history_click.borrow_mut().push(surface);
                let ctx = cairo::Context::new(surface).expect("Failed to create context");

                match &*current_tool_click.borrow() {
                    Tool::Brush(brush) => {
                        brush.draw(&ctx, x, y);
                    }
                    Tool::Eraser(eraser) => {
                        eraser.erase(&ctx, x, y);
                    }
                }
                
                // Mark as changed
                if !*has_changes_click.borrow() {
                    *has_changes_click.borrow_mut() = true;
                    if let Some(ref callback) = *on_changed_click.borrow() {
                        callback(true);
                    }
                }

                *hover_pos_click.borrow_mut() = Some((x, y));
                drawing_area_click.queue_draw();
            }
        });
        
        self.drawing_area.add_controller(click);
    }
    
    pub fn widget(&self) -> &ScrolledWindow {
        &self.container
    }
    
    pub fn set_base_image(&self, image: &image::DynamicImage) {
        let (width, height) = (image.width() as i32, image.height() as i32);
        println!("[SET BASE IMAGE] Setting image {}x{}", width, height);
        
        // Store original dimensions for export
        *self.original_dimensions.borrow_mut() = Some((image.width(), image.height()));
        
        // Convert image to cairo surface using create_for_data with proper stride
        let stride = cairo::Format::ARgb32.stride_for_width(width as u32).expect("Invalid width");
        println!("[SET BASE IMAGE] Calculated stride: {} for width {}", stride, width);
        
        // Convert DynamicImage to RGBA buffer and draw to surface
        let rgba = image.to_rgba8();
        let img_data = rgba.as_raw();
        
        // Create a buffer with Cairo's ARGB32 format (BGRA premultiplied)
        // Use stride (row bytes) which may be larger than width * 4 due to alignment
        let mut cairo_data = vec![0u8; (stride * height) as usize];
        
        for y in 0..height as usize {
            for x in 0..width as usize {
                let src_offset = (y * width as usize + x) * 4;
                let dst_offset = (y * stride as usize) + (x * 4);
                
                let r = img_data[src_offset] as u32;
                let g = img_data[src_offset + 1] as u32;
                let b = img_data[src_offset + 2] as u32;
                let a = img_data[src_offset + 3] as u32;
                
                // Premultiply alpha for ARGB32
                let premul_r = (r * a / 255) as u8;
                let premul_g = (g * a / 255) as u8;
                let premul_b = (b * a / 255) as u8;
                
                // Cairo ARGB32 format: B, G, R, A in native endian
                cairo_data[dst_offset] = premul_b;
                cairo_data[dst_offset + 1] = premul_g;
                cairo_data[dst_offset + 2] = premul_r;
                cairo_data[dst_offset + 3] = a as u8;
            }
        }
        
        // Create surface from data buffer
        let surface = ImageSurface::create_for_data(
            cairo_data,
            cairo::Format::ARgb32,
            width,
            height,
            stride
        ).expect("Failed to create surface from data");
        println!("[SET BASE IMAGE] Base surface created: {}x{}", surface.width(), surface.height());
        
        *self.base_surface.borrow_mut() = Some(surface);
        println!("[SET BASE IMAGE] Base surface stored in RefCell");
        
        // Create overlay surface with same dimensions
        let overlay_stride = cairo::Format::ARgb32.stride_for_width(width as u32).unwrap();
        let overlay_data = vec![0u8; (overlay_stride * height) as usize];
        let overlay = ImageSurface::create_for_data(
            overlay_data,
            cairo::Format::ARgb32,
            width,
            height,
            overlay_stride
        ).expect("Failed to create overlay surface");
        *self.overlay_surface.borrow_mut() = Some(overlay);
        println!("[SET BASE IMAGE] Overlay surface created and stored");
        
        // Clear history and changes
        self.history.borrow_mut().clear();
        *self.last_pos.borrow_mut() = None;
        *self.hover_pos.borrow_mut() = None;
        *self.drag_happened.borrow_mut() = false;
        *self.has_changes.borrow_mut() = false;
        if let Some(ref callback) = *self.on_changed.borrow() {
            callback(false);
        }
        
        self.drawing_area.set_content_width(width);
        self.drawing_area.set_content_height(height);
        println!("[SET BASE IMAGE] Drawing area size set to {}x{}", width, height);
        self.drawing_area.queue_draw();
        println!("[SET BASE IMAGE] queue_draw() called");
    }
    
    pub fn clear_overlay(&self) {
        if let Some(ref surface) = *self.overlay_surface.borrow() {
            let ctx = cairo::Context::new(surface).expect("Failed to create context");
            ctx.set_operator(cairo::Operator::Clear);
            ctx.paint().ok();
            ctx.set_operator(cairo::Operator::Over);
            self.drawing_area.queue_draw();
            
            // Mark as changed
            if !*self.has_changes.borrow() {
                *self.has_changes.borrow_mut() = true;
                if let Some(ref callback) = *self.on_changed.borrow() {
                    callback(true);
                }
            }
        }
    }
    
    pub fn undo(&self) {
        let mut history = self.history.borrow_mut();
        if history.undo() {
            drop(history);
            if let Some(ref mut surface) = *self.overlay_surface.borrow_mut() {
                self.history.borrow().restore_to_surface(surface);
                self.drawing_area.queue_draw();
                
                // Update change status
                let has_changes = !self.history.borrow().is_at_start();
                if *self.has_changes.borrow() != has_changes {
                    *self.has_changes.borrow_mut() = has_changes;
                    if let Some(ref callback) = *self.on_changed.borrow() {
                        callback(has_changes);
                    }
                }
            }
        }
    }

    pub fn redo(&self) {
        let mut history = self.history.borrow_mut();
        if history.redo() {
            drop(history);
            if let Some(ref mut surface) = *self.overlay_surface.borrow_mut() {
                self.history.borrow().restore_to_surface(surface);
                self.drawing_area.queue_draw();
                
                // Update change status
                let has_changes = !self.history.borrow().is_at_start();
                if *self.has_changes.borrow() != has_changes {
                    *self.has_changes.borrow_mut() = has_changes;
                    if let Some(ref callback) = *self.on_changed.borrow() {
                        callback(has_changes);
                    }
                }
            }
        }
    }
    
    pub fn can_undo(&self) -> bool {
        self.history.borrow().can_undo()
    }
    
    pub fn can_redo(&self) -> bool {
        self.history.borrow().can_redo()
    }
    
    pub fn has_changes(&self) -> bool {
        *self.has_changes.borrow()
    }
    
    pub fn set_brush_size(&self, size: f64) {
        match &mut *self.current_tool.borrow_mut() {
            Tool::Brush(brush) => brush.size = size,
            Tool::Eraser(eraser) => eraser.size = size,
        }
        self.drawing_area.queue_draw();
    }

    pub fn set_brush_color(&self, r: f64, g: f64, b: f64, a: f64) {
        if let Tool::Brush(ref mut brush) = *self.current_tool.borrow_mut() {
            brush.color = (r, g, b, a);
        }
        self.drawing_area.queue_draw();
    }

    pub fn set_brush_shape(&self, shape: crate::editor::brush::BrushShape) {
        match *self.current_tool.borrow_mut() {
            Tool::Brush(ref mut brush) => brush.shape = shape,
            Tool::Eraser(ref mut eraser) => eraser.shape = shape,
        }
        self.drawing_area.queue_draw();
    }

    pub fn set_tool(&self, tool: Tool) {
        *self.current_tool.borrow_mut() = tool;
        self.drawing_area.queue_draw();
    }
    
    pub fn get_current_tool_type(&self) -> &'static str {
        match &*self.current_tool.borrow() {
            Tool::Brush(_) => "brush",
            Tool::Eraser(_) => "eraser",
        }
    }
    
    pub fn get_brush_size(&self) -> f64 {
        match &*self.current_tool.borrow() {
            Tool::Brush(brush) => brush.size,
            Tool::Eraser(eraser) => eraser.size,
        }
    }
    
    pub fn set_on_changed_callback<F: Fn(bool) + 'static>(&self, callback: F) {
        *self.on_changed.borrow_mut() = Some(std::boxed::Box::new(callback));
    }
    
    pub fn get_composite_image(&self) -> Option<image::DynamicImage> {
        let base = self.base_surface.borrow();
        let overlay = self.overlay_surface.borrow();
        let dimensions = self.original_dimensions.borrow();
        
        let (base_surf, overlay_surf) = match (&*base, &*overlay) {
            (Some(b), Some(o)) => (b, o),
            _ => return None,
        };
        
        let (orig_w, orig_h) = dimensions.as_ref()?;
        
        let width = base_surf.width();
        let height = base_surf.height();
        
        // Create a new image to composite both layers
        let mut composite = surface_to_rgba_image(base_surf)?;
        let overlay_image = surface_to_rgba_image(overlay_surf)?;

        for y in 0..height as u32 {
            for x in 0..width as u32 {
                let overlay_pixel = overlay_image.get_pixel(x, y);
                if overlay_pixel[3] > 0 {
                    let base_pixel = composite.get_pixel(x, y);
                    let blended = blend_pixels(*base_pixel, *overlay_pixel);
                    composite.put_pixel(x, y, blended);
                }
            }
        }
        
        // Resize to original dimensions if needed
        let mut img = image::DynamicImage::ImageRgba8(composite);
        if img.width() != *orig_w || img.height() != *orig_h {
            img = crate::image_processor::resizer::resize_to_exact_fit(&img, *orig_w, *orig_h);
        }
        
        Some(img)
    }
    
    pub fn get_original_dimensions(&self) -> Option<(u32, u32)> {
        *self.original_dimensions.borrow()
    }
    
    pub fn clear_changes(&self) {
        *self.has_changes.borrow_mut() = false;
        self.history.borrow_mut().clear();
        if let Some(ref callback) = *self.on_changed.borrow() {
            callback(false);
        }
    }

    pub fn mark_changed(&self) {
        if !*self.has_changes.borrow() {
            *self.has_changes.borrow_mut() = true;
            if let Some(ref callback) = *self.on_changed.borrow() {
                callback(true);
            }
        }
    }
}

fn blend_pixels(bottom: image::Rgba<u8>, top: image::Rgba<u8>) -> image::Rgba<u8> {
    let a_top = top[3] as f32 / 255.0;
    let a_bottom = bottom[3] as f32 / 255.0;
    
    let a_out = a_top + a_bottom * (1.0 - a_top);
    if a_out == 0.0 {
        return image::Rgba([0, 0, 0, 0]);
    }
    
    let r = ((top[0] as f32 * a_top + bottom[0] as f32 * a_bottom * (1.0 - a_top)) / a_out) as u8;
    let g = ((top[1] as f32 * a_top + bottom[1] as f32 * a_bottom * (1.0 - a_top)) / a_out) as u8;
    let b = ((top[2] as f32 * a_top + bottom[2] as f32 * a_bottom * (1.0 - a_top)) / a_out) as u8;
    let a = (a_out * 255.0) as u8;
    
    image::Rgba([r, g, b, a])
}

fn draw_tool_preview(
    ctx: &cairo::Context,
    x: f64,
    y: f64,
    size: f64,
    shape: BrushShape,
    color: Option<(f64, f64, f64, f64)>,
) {
    let (r, g, b, a) = color.unwrap_or((1.0, 1.0, 1.0, 1.0));
    ctx.save().ok();
    ctx.set_source_rgba(r, g, b, (a * 0.8).max(0.35));
    ctx.set_line_width(1.5);

    match shape {
        BrushShape::Circle => {
            ctx.arc(x, y, size / 2.0, 0.0, 2.0 * std::f64::consts::PI);
            ctx.stroke().ok();
        }
        BrushShape::Square => {
            let half = size / 2.0;
            ctx.rectangle(x - half, y - half, size, size);
            ctx.stroke().ok();
        }
        BrushShape::Line => {
            ctx.move_to(x, y - size / 2.0);
            ctx.line_to(x, y + size / 2.0);
            ctx.stroke().ok();
        }
    }

    ctx.restore().ok();
}

fn surface_to_rgba_image(surface: &ImageSurface) -> Option<image::RgbaImage> {
    let width = surface.width() as u32;
    let height = surface.height() as u32;
    let stride = surface.stride() as usize;

    let data: Vec<u8> = {
        let mut surface_clone = surface.clone();
        surface_clone.data().ok()?.to_vec()
    };

    let mut image = image::RgbaImage::new(width, height);

    for y in 0..height as usize {
        for x in 0..width as usize {
            let offset = y * stride + x * 4;
            let b = data[offset] as u32;
            let g = data[offset + 1] as u32;
            let r = data[offset + 2] as u32;
            let a = data[offset + 3] as u32;

            let rgba = if a > 0 {
                image::Rgba([
                    ((r * 255) / a).min(255) as u8,
                    ((g * 255) / a).min(255) as u8,
                    ((b * 255) / a).min(255) as u8,
                    a as u8,
                ])
            } else {
                image::Rgba([0, 0, 0, 0])
            };

            image.put_pixel(x as u32, y as u32, rgba);
        }
    }

    Some(image)
}

impl Default for EditorCanvas {
    fn default() -> Self {
        Self::new()
    }
}
