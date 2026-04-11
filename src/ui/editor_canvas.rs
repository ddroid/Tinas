use gtk4::prelude::*;
use gtk4::{DrawingArea, EventControllerMotion, GestureDrag, GestureClick, Box, Orientation, ScrolledWindow};
use cairo::ImageSurface;
use std::cell::RefCell;
use std::rc::Rc;
use std::time::Instant;

use crate::editor::brush::{Brush, BrushShape, Eraser};
use crate::editor::history::History;

const DRAW_THROTTLE_MS: u128 = 16; // ~60fps max redraw rate for hover preview

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
    last_pos: Rc<RefCell<Option<(f64, f64)>>>,
    hover_pos: Rc<RefCell<Option<(f64, f64)>>>,
    drag_happened: Rc<RefCell<bool>>,
    original_dimensions: Rc<RefCell<Option<(u32, u32)>>>,
    has_changes: Rc<RefCell<bool>>,
    on_changed: Rc<RefCell<Option<std::boxed::Box<dyn Fn(bool)>>>>,
    last_draw_time: Rc<RefCell<Instant>>,
    load_generation: Rc<RefCell<u64>>,
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
            last_pos: Rc::new(RefCell::new(None)),
            hover_pos: Rc::new(RefCell::new(None)),
            drag_happened: Rc::new(RefCell::new(false)),
            original_dimensions: Rc::new(RefCell::new(None)),
            has_changes: Rc::new(RefCell::new(false)),
            on_changed: Rc::new(RefCell::new(None)),
            last_draw_time: Rc::new(RefCell::new(Instant::now())),
            load_generation: Rc::new(RefCell::new(0)),
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
            let has_base = if let Some(ref surface) = *base_surface.borrow() {
                ctx.set_source_surface(surface, 0.0, 0.0).ok();
                ctx.paint().ok();
                true
            } else {
                // Show loading indicator while async image decode is in progress
                ctx.set_source_rgb(0.6, 0.6, 0.6);
                ctx.select_font_face("Sans", cairo::FontSlant::Normal, cairo::FontWeight::Normal);
                ctx.set_font_size(14.0);
                ctx.move_to(20.0, 50.0);
                let _ = ctx.show_text("Loading...");
                false
            };
            
            if !has_base {
                return;
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
        let last_draw_time_motion = self.last_draw_time.clone();
        motion.connect_motion(move |_controller, x, y| {
            *hover_pos_motion.borrow_mut() = Some((x, y));
            let now = Instant::now();
            let elapsed = now.duration_since(*last_draw_time_motion.borrow()).as_millis();
            if elapsed >= DRAW_THROTTLE_MS {
                *last_draw_time_motion.borrow_mut() = now;
                drawing_area_motion.queue_draw();
            }
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
    
    fn apply_surface_data(&self, cairo_data: Vec<u8>, width: i32, height: i32, stride: i32) {
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
        
        // Adapt history depth for large images to avoid excessive memory usage
        let pixel_count = (width as u64) * (height as u64);
        let max_history = if pixel_count > 2_000_000 {
            15  // ~1080p+: ~120MB max
        } else if pixel_count > 500_000 {
            30  // medium images
        } else {
            50  // small images
        };
        *self.history.borrow_mut() = History::new(max_history);
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

    /// Convert RGBA image data to Cairo BGRA premultiplied format.
    /// This is the CPU-heavy part that can be done off the main thread.
    fn convert_rgba_to_cairo(image: &image::DynamicImage) -> (Vec<u8>, i32, i32, i32) {
        let (width, height) = (image.width() as i32, image.height() as i32);
        let stride = cairo::Format::ARgb32.stride_for_width(width as u32).expect("Invalid width");
        let rgba = image.to_rgba8();
        let img_data = rgba.as_raw();
        
        let mut cairo_data = vec![0u8; (stride * height) as usize];
        let row_bytes = width as usize * 4;
        let stride_usize = stride as usize;
        
        for y in 0..height as usize {
            let src_row = &img_data[y * row_bytes..(y * row_bytes + row_bytes)];
            let dst_row = &mut cairo_data[y * stride_usize..y * stride_usize + row_bytes];
            for (src_px, dst_px) in src_row.chunks_exact(4).zip(dst_row.chunks_exact_mut(4)) {
                let r = src_px[0] as u32;
                let g = src_px[1] as u32;
                let b = src_px[2] as u32;
                let a = src_px[3] as u32;
                dst_px[0] = (b * a / 255) as u8;
                dst_px[1] = (g * a / 255) as u8;
                dst_px[2] = (r * a / 255) as u8;
                dst_px[3] = a as u8;
            }
        }
        
        (cairo_data, width, height, stride)
    }

    pub fn set_base_image(&self, image: &image::DynamicImage) {
        let (width, height) = (image.width() as i32, image.height() as i32);
        println!("[SET BASE IMAGE] Setting image {}x{}", width, height);
        
        // Store original dimensions for export
        *self.original_dimensions.borrow_mut() = Some((image.width(), image.height()));
        
        let stride = cairo::Format::ARgb32.stride_for_width(width as u32).expect("Invalid width");
        println!("[SET BASE IMAGE] Calculated stride: {} for width {}", stride, width);
        
        let (cairo_data, w, h, s) = Self::convert_rgba_to_cairo(image);
        self.apply_surface_data(cairo_data, w, h, s);
    }

    /// Asynchronously decode image bytes and set the base image without blocking the UI.
    /// For small images (<500K pixels), falls back to synchronous loading.
    pub fn set_base_image_async(&self, data: Vec<u8>) {
        // Bump generation to invalidate any in-flight async loads
        let current_gen = {
            let mut g = self.load_generation.borrow_mut();
            *g += 1;
            *g
        };

        // Quick peek at image dimensions to decide sync vs async
        let needs_async = image::ImageReader::new(std::io::Cursor::new(&data))
            .with_guessed_format()
            .ok()
            .and_then(|r| r.into_dimensions().ok())
            .map(|(w, h)| (w as u64) * (h as u64) > 500_000)
            .unwrap_or(false);

        if !needs_async {
            // Small image: decode synchronously (fast enough)
            match image::load_from_memory(&data) {
                Ok(img) => {
                    let (w, h) = (img.width(), img.height());
                    println!("[DEBUG] Image loaded successfully: {}x{}", w, h);
                    *self.original_dimensions.borrow_mut() = Some((w, h));
                    let (cairo_data, cw, ch, cs) = Self::convert_rgba_to_cairo(&img);
                    self.apply_surface_data(cairo_data, cw, ch, cs);
                    println!("[DEBUG] Image set to canvas");
                }
                Err(e) => eprintln!("[DEBUG] Failed to load image: {}", e),
            }
            return;
        }

        // Large image: show loading placeholder immediately, then process in background
        self.show_loading();

        let load_generation = Rc::clone(&self.load_generation);
        let base_surface = Rc::clone(&self.base_surface);
        let overlay_surface = Rc::clone(&self.overlay_surface);
        let history = Rc::clone(&self.history);
        let last_pos = Rc::clone(&self.last_pos);
        let hover_pos = Rc::clone(&self.hover_pos);
        let drag_happened = Rc::clone(&self.drag_happened);
        let has_changes = Rc::clone(&self.has_changes);
        let on_changed = Rc::clone(&self.on_changed);
        let drawing_area = self.drawing_area.clone();
        let original_dimensions = Rc::clone(&self.original_dimensions);

        // Use spawn_future_local so the Rc values stay on the main thread.
        // The heavy decode+convert work runs on a blocking thread pool via gio::spawn_blocking.
        glib::spawn_future_local(async move {
            let result = gtk4::gio::spawn_blocking(move || {
                match image::load_from_memory(&data) {
                    Ok(img) => {
                        let (w, h) = (img.width(), img.height());
                        println!("[ASYNC] Image decoded: {}x{}", w, h);
                        let (cairo_data, cw, ch, cs) = Self::convert_rgba_to_cairo(&img);
                        Some((cairo_data, cw, ch, cs, w, h))
                    }
                    Err(e) => {
                        eprintln!("[ASYNC] Failed to decode image: {}", e);
                        None
                    }
                }
            }).await;

            let Some(Some((cairo_data, width, height, stride, orig_w, orig_h))) = result.ok() else {
                return;
            };

            // Check if this load is still current
            if *load_generation.borrow() != current_gen {
                println!("[ASYNC] Stale load discarded (gen {} vs current {})", current_gen, *load_generation.borrow());
                return;
            }

            println!("[ASYNC] Applying surface {}x{}", width, height);

            *original_dimensions.borrow_mut() = Some((orig_w, orig_h));

            // Create base surface
            let surface = ImageSurface::create_for_data(
                cairo_data,
                cairo::Format::ARgb32,
                width,
                height,
                stride,
            ).expect("Failed to create surface from data");
            *base_surface.borrow_mut() = Some(surface);

            // Create overlay surface
            let overlay_stride = cairo::Format::ARgb32.stride_for_width(width as u32).unwrap();
            let overlay_data_buf = vec![0u8; (overlay_stride * height) as usize];
            let overlay = ImageSurface::create_for_data(
                overlay_data_buf,
                cairo::Format::ARgb32,
                width,
                height,
                overlay_stride,
            ).expect("Failed to create overlay surface");
            *overlay_surface.borrow_mut() = Some(overlay);

            // Adapt history
            let pixel_count = (width as u64) * (height as u64);
            let max_history = if pixel_count > 2_000_000 {
                15
            } else if pixel_count > 500_000 {
                30
            } else {
                50
            };
            *history.borrow_mut() = History::new(max_history);
            *last_pos.borrow_mut() = None;
            *hover_pos.borrow_mut() = None;
            *drag_happened.borrow_mut() = false;
            *has_changes.borrow_mut() = false;
            if let Some(ref callback) = *on_changed.borrow() {
                callback(false);
            }

            drawing_area.set_content_width(width);
            drawing_area.set_content_height(height);
            drawing_area.queue_draw();
            println!("[ASYNC] Image applied to canvas");
        });
    }

    /// Show a loading placeholder while an image is being decoded asynchronously.
    fn show_loading(&self) {
        *self.base_surface.borrow_mut() = None;
        *self.overlay_surface.borrow_mut() = None;
        self.drawing_area.set_content_width(200);
        self.drawing_area.set_content_height(100);
        self.drawing_area.queue_draw();
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
            _ => {
                println!("[COMPOSITE] FAIL: base={} overlay={}", base.is_some(), overlay.is_some());
                return None;
            }
        };
        
        let (orig_w, orig_h) = match dimensions.as_ref() {
            Some(d) => d,
            None => {
                println!("[COMPOSITE] FAIL: original_dimensions is None");
                return None;
            }
        };
        
        let width = base_surf.width();
        let height = base_surf.height();
        println!("[COMPOSITE] surfaces: {}x{}, orig: {}x{}", width, height, orig_w, orig_h);
        
        // Use cairo to composite base + overlay (GPU-accelerated, much faster than per-pixel)
        let composite_surface = ImageSurface::create(cairo::Format::ARgb32, width, height)
            .expect("Failed to create composite surface");
        {
            let ctx = cairo::Context::new(&composite_surface).expect("Failed to create context");
            ctx.set_source_surface(base_surf, 0.0, 0.0).ok()?;
            ctx.paint().ok()?;
            ctx.set_source_surface(overlay_surf, 0.0, 0.0).ok()?;
            ctx.paint().ok()?;
        }
        composite_surface.flush();
        
        // Convert composited surface to RGBA image
        let composite = surface_to_rgba_image(&composite_surface)?;
        
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

    /// Clear the unsaved-changes flag without discarding the undo history.
    pub fn mark_saved(&self) {
        *self.has_changes.borrow_mut() = false;
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

    surface.flush();

    // Copy to a new owned surface to avoid NonExclusive errors
    // from surfaces created with create_for_data.
    let mut copy = ImageSurface::create(cairo::Format::ARgb32, width as i32, height as i32)
        .expect("Failed to create copy surface");
    {
        let ctx = cairo::Context::new(&copy).expect("Failed to create context");
        ctx.set_source_surface(surface, 0.0, 0.0).ok()?;
        ctx.paint().ok()?;
    }
    copy.flush();

    let stride = copy.stride() as usize;
    let row_bytes = width as usize * 4;
    let data: Vec<u8> = {
        match copy.data() {
            Ok(d) => d.to_vec(),
            Err(e) => {
                println!("[SURFACE] data() FAILED even on copy: {:?} (surface {}x{}, stride={})", e, width, height, stride);
                return None;
            }
        }
    };

    let mut pixels = vec![0u8; (width * height * 4) as usize];

    for y in 0..height as usize {
        let src_row = &data[y * stride..y * stride + row_bytes];
        let dst_row = &mut pixels[y * row_bytes..y * row_bytes + row_bytes];
        for (src_px, dst_px) in src_row.chunks_exact(4).zip(dst_row.chunks_exact_mut(4)) {
            let b = src_px[0] as u32;
            let g = src_px[1] as u32;
            let r = src_px[2] as u32;
            let a = src_px[3] as u32;
            if a > 0 {
                dst_px[0] = ((r * 255) / a).min(255) as u8;
                dst_px[1] = ((g * 255) / a).min(255) as u8;
                dst_px[2] = ((b * 255) / a).min(255) as u8;
                dst_px[3] = a as u8;
            }
            // else: already zero-initialized
        }
    }

    image::RgbaImage::from_raw(width, height, pixels)
}

impl Default for EditorCanvas {
    fn default() -> Self {
        Self::new()
    }
}
