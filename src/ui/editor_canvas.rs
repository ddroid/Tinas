use gtk4::prelude::*;
use gtk4::{DrawingArea, GestureDrag, GestureClick, Box, Orientation, ScrolledWindow};
use cairo::ImageSurface;
use std::cell::RefCell;
use std::rc::Rc;

use crate::editor::brush::{Brush, Eraser};
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
            current_tool: Rc::new(RefCell::new(Tool::Brush(Brush::new(8.0, (0.0, 0.0, 0.0, 1.0))))),
            history: Rc::new(RefCell::new(History::new(50))),
            last_pos: RefCell::new(None),
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
        
        // Clone for second closure
        let overlay_surface_clone = overlay_surface.clone();
        let last_pos_clone = last_pos.clone();
        let current_tool_clone = Rc::clone(&current_tool);
        let has_changes_clone = has_changes.clone();
        let on_changed_clone = on_changed.clone();
        let drawing_area_clone = drawing_area.clone();
        
        // Clone for click handler
        let overlay_surface_click = overlay_surface.clone();
        let current_tool_click = Rc::clone(&current_tool);
        let history_click = Rc::clone(&history);
        let has_changes_click = has_changes.clone();
        let on_changed_click = on_changed.clone();
        
        // Drag gesture for drawing
        let drag = GestureDrag::new();
        drag.set_button(gtk4::gdk::BUTTON_PRIMARY);
        
        drag.connect_drag_begin(move |_gesture, x, y| {
            println!("[DEBUG] Drag begin at ({}, {})", x, y);
            
            // Save state before drawing
            if let Some(ref mut surface) = *overlay_surface.borrow_mut() {
                history.borrow_mut().push(surface);
            }
            *last_pos.borrow_mut() = Some((x, y));
            
            // Mark as changed
            if !*has_changes.borrow() {
                *has_changes.borrow_mut() = true;
                if let Some(ref callback) = *on_changed.borrow() {
                    callback(true);
                }
            }
            
            drawing_area.queue_draw();
        });
        
        drag.connect_drag_update(move |_gesture, offset_x, offset_y| {
            if let Some((start_x, start_y)) = _gesture.start_point() {
                let current_x = start_x + offset_x;
                let current_y = start_y + offset_y;
                
                // Clone the surface to get mutable access for drawing
                let surface_clone = overlay_surface_clone.borrow().clone();
                if let Some(ref surface) = surface_clone {
                    let ctx = cairo::Context::new(surface).expect("Failed to create context");
                    
                    // Draw line from last position to current
                    if let Some((last_x, last_y)) = *last_pos_clone.borrow() {
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
                    } else {
                        // First point - draw single stroke
                        match &*current_tool_clone.borrow() {
                            Tool::Brush(brush) => {
                                brush.draw(&ctx, current_x, current_y);
                            }
                            Tool::Eraser(eraser) => {
                                eraser.erase(&ctx, current_x, current_y);
                            }
                        }
                    }
                    
                    *last_pos_clone.borrow_mut() = Some((current_x, current_y));
                    
                    // Mark as changed
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
        
        self.drawing_area.add_controller(drag);
        
        // Click gesture for single clicks
        let click = GestureClick::new();
        
        click.connect_pressed(move |_gesture, _n_press, x, y| {
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
    }

    pub fn set_brush_color(&self, r: f64, g: f64, b: f64, a: f64) {
        if let Tool::Brush(ref mut brush) = *self.current_tool.borrow_mut() {
            brush.color = (r, g, b, a);
        }
    }

    pub fn set_tool(&self, tool: Tool) {
        *self.current_tool.borrow_mut() = tool;
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
        let mut composite = image::RgbaImage::new(width as u32, height as u32);
        
        // Extract base surface pixels
        // We need to clone the data since data() requires mutable borrow
        let base_data: Vec<u8> = {
            let mut surf = base_surf.clone();
            surf.data().map(|d| d.to_vec()).unwrap_or_default()
        };
        
        for (i, chunk) in base_data.chunks_exact(4).enumerate() {
            let x = (i as u32) % (width as u32);
            let y = (i as u32) / (width as u32);
            if y < height as u32 {
                // Cairo ARGB32 to RGBA (unpremultiply)
                let b = chunk[0] as u32;
                let g = chunk[1] as u32;
                let r = chunk[2] as u32;
                let a = chunk[3] as u32;
                
                if a > 0 {
                    let unpremul_r = ((r * 255) / a).min(255) as u8;
                    let unpremul_g = ((g * 255) / a).min(255) as u8;
                    let unpremul_b = ((b * 255) / a).min(255) as u8;
                    composite.put_pixel(x, y, image::Rgba([unpremul_r, unpremul_g, unpremul_b, a as u8]));
                } else {
                    composite.put_pixel(x, y, image::Rgba([0, 0, 0, 0]));
                }
            }
        }
        
        // Blend overlay on top
        let overlay_data: Vec<u8> = {
            let mut surf = overlay_surf.clone();
            surf.data().map(|d| d.to_vec()).unwrap_or_default()
        };
        
        for (i, chunk) in overlay_data.chunks_exact(4).enumerate() {
            let x = (i as u32) % (width as u32);
            let y = (i as u32) / (width as u32);
            if y < height as u32 {
                let a = chunk[3] as u32;
                if a > 0 {
                    let b = chunk[0] as u32;
                    let g = chunk[1] as u32;
                    let r = chunk[2] as u32;
                    
                    let unpremul_r = ((r * 255) / a).min(255) as u8;
                    let unpremul_g = ((g * 255) / a).min(255) as u8;
                    let unpremul_b = ((b * 255) / a).min(255) as u8;
                    
                    let overlay_pixel = image::Rgba([unpremul_r, unpremul_g, unpremul_b, a as u8]);
                    let base_pixel = composite.get_pixel(x, y);
                    let blended = blend_pixels(*base_pixel, overlay_pixel);
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

impl Default for EditorCanvas {
    fn default() -> Self {
        Self::new()
    }
}
