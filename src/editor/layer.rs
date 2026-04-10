use cairo::ImageSurface;
use std::cell::RefCell;
use std::rc::Rc;

/// A single layer in the image editor
pub struct Layer {
    pub id: usize,
    pub name: String,
    pub visible: RefCell<bool>,
    pub opacity: RefCell<f64>, // 0.0 to 1.0
    pub surface: RefCell<Option<ImageSurface>>,
    pub locked: RefCell<bool>,
    pub blend_mode: RefCell<BlendMode>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum BlendMode {
    Normal,
    Multiply,
    Screen,
    Overlay,
    Add,
    Subtract,
}

impl BlendMode {
    pub fn as_str(&self) -> &'static str {
        match self {
            BlendMode::Normal => "Normal",
            BlendMode::Multiply => "Multiply",
            BlendMode::Screen => "Screen",
            BlendMode::Overlay => "Overlay",
            BlendMode::Add => "Add",
            BlendMode::Subtract => "Subtract",
        }
    }
}

impl Layer {
    pub fn new(id: usize, name: &str, width: i32, height: i32) -> Self {
        let surface = ImageSurface::create(cairo::Format::ARgb32, width, height).ok();
        
        Self {
            id,
            name: name.to_string(),
            visible: RefCell::new(true),
            opacity: RefCell::new(1.0),
            surface: RefCell::new(surface),
            locked: RefCell::new(false),
            blend_mode: RefCell::new(BlendMode::Normal),
        }
    }

    pub fn with_image(id: usize, name: &str, image: &image::DynamicImage) -> Self {
        let (width, height) = (image.width() as i32, image.height() as i32);
        let mut surface = ImageSurface::create(cairo::Format::ARgb32, width, height).ok();
        
        // Convert image data to Cairo format and draw onto surface
        if let Some(ref mut surf) = surface {
            // Convert RGBA to Cairo ARGB
            let rgba = image.to_rgba8();
            let stride = surf.stride() as usize;
            let mut cairo_data = vec![0u8; stride * height as usize];
            
            for y in 0..height as usize {
                for x in 0..width as usize {
                    let dst_offset = (y * stride) + (x * 4);
                    let pixel = rgba.get_pixel(x as u32, y as u32);
                    let [r, g, b, a] = pixel.0;
                    
                    // Convert to Cairo ARGB32 (premultiplied)
                    let alpha = a as f32 / 255.0;
                    let premul_r = ((r as f32 * alpha) as u32).min(255) as u8;
                    let premul_g = ((g as f32 * alpha) as u32).min(255) as u8;
                    let premul_b = ((b as f32 * alpha) as u32).min(255) as u8;
                    
                    cairo_data[dst_offset] = premul_b;
                    cairo_data[dst_offset + 1] = premul_g;
                    cairo_data[dst_offset + 2] = premul_r;
                    cairo_data[dst_offset + 3] = a;
                }
            }
            
            // Copy data to surface
            if let Ok(mut data) = surf.data() {
                data.copy_from_slice(&cairo_data);
            }
            surf.mark_dirty();
        }
        
        Self {
            id,
            name: name.to_string(),
            visible: RefCell::new(true),
            opacity: RefCell::new(1.0),
            surface: RefCell::new(surface),
            locked: RefCell::new(false),
            blend_mode: RefCell::new(BlendMode::Normal),
        }
    }

    pub fn set_visible(&self, visible: bool) {
        *self.visible.borrow_mut() = visible;
    }

    pub fn is_visible(&self) -> bool {
        *self.visible.borrow()
    }

    pub fn set_opacity(&self, opacity: f64) {
        *self.opacity.borrow_mut() = opacity.clamp(0.0, 1.0);
    }

    pub fn get_opacity(&self) -> f64 {
        *self.opacity.borrow()
    }

    pub fn set_locked(&self, locked: bool) {
        *self.locked.borrow_mut() = locked;
    }

    pub fn is_locked(&self) -> bool {
        *self.locked.borrow()
    }

    pub fn set_blend_mode(&self, mode: BlendMode) {
        *self.blend_mode.borrow_mut() = mode;
    }

    pub fn get_blend_mode(&self) -> BlendMode {
        *self.blend_mode.borrow()
    }

    pub fn rename(&mut self, name: &str) {
        self.name = name.to_string();
    }

    pub fn get_context(&self) -> Option<cairo::Context> {
        self.surface.borrow().as_ref()
            .and_then(|s| cairo::Context::new(s).ok())
    }

    pub fn clear(&self) {
        if let Some(ref surface) = *self.surface.borrow() {
            let ctx = cairo::Context::new(surface).expect("Failed to create context");
            ctx.set_operator(cairo::Operator::Clear);
            ctx.paint().expect("Failed to clear layer");
            ctx.set_operator(cairo::Operator::Over);
            surface.mark_dirty();
        }
    }

    pub fn duplicate(&self, new_id: usize) -> Self {
        let surface = self.surface.borrow();
        let new_surface = surface.as_ref().map(|s| {
            let mut new_surf = ImageSurface::create(cairo::Format::ARgb32, s.width(), s.height()).unwrap();
            // Clone the surface by drawing it onto the new one
            let ctx = cairo::Context::new(&new_surf).expect("Failed to create context");
            ctx.set_source_surface(s, 0.0, 0.0).expect("Failed to set source");
            ctx.paint().expect("Failed to paint");
            new_surf.mark_dirty();
            new_surf
        });

        Self {
            id: new_id,
            name: format!("{} Copy", self.name),
            visible: RefCell::new(self.is_visible()),
            opacity: RefCell::new(self.get_opacity()),
            surface: RefCell::new(new_surface),
            locked: RefCell::new(self.is_locked()),
            blend_mode: RefCell::new(self.get_blend_mode()),
        }
    }
}

/// Manages multiple layers
pub struct LayerManager {
    layers: RefCell<Vec<Rc<Layer>>>,
    active_layer_id: RefCell<Option<usize>>,
    next_id: RefCell<usize>,
    width: i32,
    height: i32,
}

impl LayerManager {
    pub fn new(width: i32, height: i32) -> Self {
        let mut manager = Self {
            layers: RefCell::new(Vec::new()),
            active_layer_id: RefCell::new(None),
            next_id: RefCell::new(1),
            width,
            height,
        };
        
        // Create a default background layer
        manager.add_layer("Background");
        
        manager
    }

    pub fn add_layer(&self, name: &str) -> Rc<Layer> {
        let id = *self.next_id.borrow();
        *self.next_id.borrow_mut() += 1;
        
        let layer = Rc::new(Layer::new(id, name, self.width, self.height));
        self.layers.borrow_mut().push(layer.clone());
        
        // Set as active if it's the first layer
        if self.active_layer_id.borrow().is_none() {
            *self.active_layer_id.borrow_mut() = Some(id);
        }
        
        layer
    }

    pub fn add_layer_with_image(&self, name: &str, image: &image::DynamicImage) -> Rc<Layer> {
        let id = *self.next_id.borrow();
        *self.next_id.borrow_mut() += 1;
        
        let layer = Rc::new(Layer::with_image(id, name, image));
        self.layers.borrow_mut().push(layer.clone());
        
        if self.active_layer_id.borrow().is_none() {
            *self.active_layer_id.borrow_mut() = Some(id);
        }
        
        layer
    }

    pub fn delete_layer(&self, id: usize) -> bool {
        let mut layers = self.layers.borrow_mut();
        let index = layers.iter().position(|l| l.id == id);
        
        if let Some(idx) = index {
            // Don't delete if it's the only layer
            if layers.len() <= 1 {
                return false;
            }
            
            layers.remove(idx);
            
            // Update active layer if needed
            if *self.active_layer_id.borrow() == Some(id) {
                *self.active_layer_id.borrow_mut() = layers.first().map(|l| l.id);
            }
            
            return true;
        }
        
        false
    }

    pub fn duplicate_layer(&self, id: usize) -> Option<Rc<Layer>> {
        let layers = self.layers.borrow();
        let layer = layers.iter().find(|l| l.id == id)?.clone();
        
        let new_id = *self.next_id.borrow();
        *self.next_id.borrow_mut() += 1;
        
        let new_layer = Rc::new(layer.duplicate(new_id));
        
        // Insert after the original layer
        let idx = layers.iter().position(|l| l.id == id).unwrap_or(0);
        drop(layers); // Drop borrow before modifying
        
        self.layers.borrow_mut().insert(idx + 1, new_layer.clone());
        
        Some(new_layer)
    }

    pub fn move_layer(&self, id: usize, new_index: usize) -> bool {
        let mut layers = self.layers.borrow_mut();
        let old_index = layers.iter().position(|l| l.id == id);
        
        if let Some(old_idx) = old_index {
            let new_idx = new_index.min(layers.len() - 1);
            if old_idx != new_idx {
                let layer = layers.remove(old_idx);
                layers.insert(new_idx, layer);
            }
            return true;
        }
        
        false
    }

    pub fn move_layer_up(&self, id: usize) -> bool {
        let index = self.layers.borrow().iter().position(|l| l.id == id);
        if let Some(idx) = index {
            return self.move_layer(id, idx + 1);
        }
        false
    }

    pub fn move_layer_down(&self, id: usize) -> bool {
        let index = self.layers.borrow().iter().position(|l| l.id == id);
        if let Some(idx) = index {
            if idx > 0 {
                return self.move_layer(id, idx - 1);
            }
        }
        false
    }

    pub fn set_active_layer(&self, id: usize) -> bool {
        let exists = self.layers.borrow().iter().any(|l| l.id == id);
        if exists {
            *self.active_layer_id.borrow_mut() = Some(id);
            true
        } else {
            false
        }
    }

    pub fn get_active_layer(&self) -> Option<Rc<Layer>> {
        let active_id = *self.active_layer_id.borrow();
        let id = active_id?;
        self.get_layer(id)
    }

    pub fn get_layer(&self, id: usize) -> Option<Rc<Layer>> {
        self.layers.borrow().iter().find(|l| l.id == id).cloned()
    }

    pub fn get_all_layers(&self) -> Vec<Rc<Layer>> {
        self.layers.borrow().clone()
    }

    pub fn get_layer_count(&self) -> usize {
        self.layers.borrow().len()
    }

    pub fn get_visible_layers(&self) -> Vec<Rc<Layer>> {
        self.layers.borrow()
            .iter()
            .filter(|l| l.is_visible())
            .cloned()
            .collect()
    }

    pub fn composite_all_layers(&self) -> Option<ImageSurface> {
        let visible_layers: Vec<_> = self.get_visible_layers();
        if visible_layers.is_empty() {
            return None;
        }

        let surface = ImageSurface::create(cairo::Format::ARgb32, self.width, self.height).ok()?;
        let ctx = cairo::Context::new(&surface).ok()?;

        // Fill with transparent background
        ctx.set_operator(cairo::Operator::Clear);
        ctx.paint().ok()?;
        ctx.set_operator(cairo::Operator::Over);

        // Composite each layer
        for layer in visible_layers {
            if let Some(ref layer_surf) = *layer.surface.borrow() {
                let opacity = layer.get_opacity();
                
                ctx.save().ok()?;
                ctx.set_source_surface(layer_surf, 0.0, 0.0).ok()?;
                ctx.paint_with_alpha(opacity).ok()?;
                ctx.restore().ok()?;
            }
        }

        surface.mark_dirty();
        Some(surface)
    }

    pub fn flatten_layers(&self) -> Option<Rc<Layer>> {
        let composite = self.composite_all_layers()?;
        
        let id = *self.next_id.borrow();
        *self.next_id.borrow_mut() += 1;
        
        let layer = Layer {
            id,
            name: "Flattened".to_string(),
            visible: RefCell::new(true),
            opacity: RefCell::new(1.0),
            surface: RefCell::new(Some(composite)),
            locked: RefCell::new(false),
            blend_mode: RefCell::new(BlendMode::Normal),
        };
        
        Some(Rc::new(layer))
    }

    pub fn clear_all(&self) {
        self.layers.borrow_mut().clear();
        *self.active_layer_id.borrow_mut() = None;
        *self.next_id.borrow_mut() = 1;
        
        // Add a new blank background layer
        self.add_layer("Background");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_layer_manager_creation() {
        let manager = LayerManager::new(100, 100);
        assert_eq!(manager.get_layer_count(), 1);
        assert!(manager.get_active_layer().is_some());
    }

    #[test]
    fn test_add_layer() {
        let manager = LayerManager::new(100, 100);
        let layer = manager.add_layer("Test Layer");
        assert_eq!(layer.name, "Test Layer");
        assert_eq!(manager.get_layer_count(), 2);
    }

    #[test]
    fn test_delete_layer() {
        let manager = LayerManager::new(100, 100);
        let layer = manager.add_layer("To Delete");
        let id = layer.id;
        
        assert!(manager.delete_layer(id));
        assert_eq!(manager.get_layer_count(), 1);
        
        // Can't delete the last layer
        let last_id = manager.get_active_layer().unwrap().id;
        assert!(!manager.delete_layer(last_id));
    }

    #[test]
    fn test_layer_visibility() {
        let manager = LayerManager::new(100, 100);
        let layer = manager.add_layer("Test");
        
        assert!(layer.is_visible());
        layer.set_visible(false);
        assert!(!layer.is_visible());
    }

    #[test]
    fn test_move_layer() {
        let manager = LayerManager::new(100, 100);
        let layer1 = manager.add_layer("Layer 1");
        let layer2 = manager.add_layer("Layer 2");
        let layer3 = manager.add_layer("Layer 3");
        
        let id = layer1.id;
        manager.move_layer_up(id);
        
        let layers = manager.get_all_layers();
        assert_eq!(layers[1].id, id);
    }
}
