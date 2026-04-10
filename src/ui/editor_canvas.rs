use gtk4::prelude::*;
use gtk4::{DrawingArea, GestureDrag, GestureClick, Box, Orientation};
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
    container: Box,
    drawing_area: DrawingArea,
    base_surface: RefCell<Option<ImageSurface>>,
    overlay_surface: RefCell<Option<ImageSurface>>,
    current_tool: Rc<RefCell<Tool>>,
    history: Rc<RefCell<History>>,
    last_pos: RefCell<Option<(f64, f64)>>,
}

impl EditorCanvas {
    pub fn new() -> Self {
        let drawing_area = DrawingArea::builder()
            .width_request(400)
            .height_request(400)
            .can_focus(true)
            .focusable(true)
            .build();
        
        let container = Box::new(Orientation::Vertical, 0);
        container.append(&drawing_area);
        
        let this = Self {
            container,
            drawing_area,
            base_surface: RefCell::new(None),
            overlay_surface: RefCell::new(None),
            current_tool: Rc::new(RefCell::new(Tool::Brush(Brush::new(8.0, (0.0, 0.0, 0.0, 1.0))))),
            history: Rc::new(RefCell::new(History::new(5))),
            last_pos: RefCell::new(None),
        };
        
        this.setup_drawing();
        this.setup_input();
        
        this
    }
    
    fn setup_drawing(&self) {
        let base_surface = self.base_surface.clone();
        let overlay_surface = self.overlay_surface.clone();
        
        self.drawing_area.set_draw_func(move |area, ctx, width, height| {
            // Clear background
            ctx.set_source_rgb(0.9, 0.9, 0.9);
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
        
        // Clone for second closure
        let overlay_surface_clone = overlay_surface.clone();
        let last_pos_clone = last_pos.clone();
        let current_tool_clone = Rc::clone(&current_tool);
        
        // Clone for click handler
        let overlay_surface_click = overlay_surface.clone();
        let current_tool_click = Rc::clone(&current_tool);
        let history_click = Rc::clone(&history);
        
        // Drag gesture for drawing
        let drag = GestureDrag::new();
        
        drag.connect_drag_begin(move |_gesture, _x, _y| {
            // Save state before drawing
            if let Some(ref mut surface) = *overlay_surface.borrow_mut() {
                history.borrow_mut().push(surface);
            }
            *last_pos.borrow_mut() = None;
            drawing_area.queue_draw();
        });
        
        drag.connect_drag_update(move |_gesture, x, y| {
            if let Some(ref surface) = *overlay_surface_clone.borrow() {
                let ctx = cairo::Context::new(surface).expect("Failed to create context");
                let (start_x, start_y) = match *last_pos_clone.borrow() {
                    Some(pos) => pos,
                    None => {
                        let start = _gesture.start_point().unwrap_or((x, y));
                        (start.0, start.1)
                    }
                };
                
                let current_x = start_x + x;
                let current_y = start_y + y;
                
                match &*current_tool_clone.borrow() {
                    Tool::Brush(brush) => {
                        brush.draw(&ctx, current_x, current_y);
                    }
                    Tool::Eraser(eraser) => {
                        eraser.erase(&ctx, current_x, current_y);
                    }
                }
                
                *last_pos_clone.borrow_mut() = Some((current_x, current_y));
                
                // Request redraw
                // This would trigger redraw via a callback in full implementation
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
            }
        });
        
        self.drawing_area.add_controller(click);
    }
    
    pub fn widget(&self) -> &Box {
        &self.container
    }
    
    pub fn set_base_image(&self, image: &image::DynamicImage) {
        let (width, height) = (image.width() as i32, image.height() as i32);
        
        // Convert image to cairo surface
        let surface = ImageSurface::create(cairo::Format::ARgb32, width, height)
            .expect("Failed to create surface");
        
        let ctx = cairo::Context::new(&surface).expect("Failed to create context");
        
        // Draw the image
        // In full implementation, convert DynamicImage pixels to cairo surface
        // For now, fill with placeholder
        ctx.set_source_rgb(1.0, 1.0, 1.0);
        ctx.paint().ok();
        
        *self.base_surface.borrow_mut() = Some(surface);
        
        // Create overlay surface
        let overlay = ImageSurface::create(cairo::Format::ARgb32, width, height)
            .expect("Failed to create overlay surface");
        *self.overlay_surface.borrow_mut() = Some(overlay);
        
        self.drawing_area.set_content_width(width);
        self.drawing_area.set_content_height(height);
        self.drawing_area.queue_draw();
    }
    
    pub fn clear_overlay(&self) {
        if let Some(ref surface) = *self.overlay_surface.borrow() {
            let ctx = cairo::Context::new(surface).expect("Failed to create context");
            ctx.set_operator(cairo::Operator::Clear);
            ctx.paint().ok();
            ctx.set_operator(cairo::Operator::Over);
            self.drawing_area.queue_draw();
        }
    }
    
    pub fn undo(&self) {
        let mut history = self.history.borrow_mut();
        if history.undo() {
            drop(history);
            if let Some(ref mut surface) = *self.overlay_surface.borrow_mut() {
                self.history.borrow().restore_to_surface(surface);
                self.drawing_area.queue_draw();
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
            }
        }
    }
    
    pub fn can_undo(&self) -> bool {
        self.history.borrow().can_undo()
    }
    
    pub fn can_redo(&self) -> bool {
        self.history.borrow().can_redo()
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
    
    pub fn get_composite_image(&self) -> Option<image::DynamicImage> {
        // Combine base and overlay surfaces and return as DynamicImage
        // Full implementation would convert cairo surfaces to image buffer
        None
    }
}

impl Default for EditorCanvas {
    fn default() -> Self {
        Self::new()
    }
}
