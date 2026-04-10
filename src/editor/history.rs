use cairo::ImageSurface;
use std::collections::VecDeque;

/// Stores pixel data for history snapshots
pub struct SurfaceData {
    width: i32,
    height: i32,
    data: Vec<u8>,
}

pub struct History {
    snapshots: VecDeque<SurfaceData>,
    current_index: usize,
    max_history: usize,
}

impl History {
    pub fn new(max_history: usize) -> Self {
        Self {
            snapshots: VecDeque::with_capacity(max_history),
            current_index: 0,
            max_history,
        }
    }

    pub fn push(&mut self, surface: &mut ImageSurface) {
        // Remove any redo states
        while self.snapshots.len() > self.current_index {
            self.snapshots.pop_back();
        }

        // Get surface data
        let width = surface.width();
        let height = surface.height();

        // Copy pixel data from surface
        if let Ok(data) = surface.data() {
            let snapshot = SurfaceData {
                width,
                height,
                data: data.to_vec(),
            };

            if self.snapshots.len() >= self.max_history {
                self.snapshots.pop_front();
            }

            self.snapshots.push_back(snapshot);
            self.current_index = self.snapshots.len();
        }
    }

    pub fn restore_to_surface(&self, surface: &mut ImageSurface) -> bool {
        if let Some(snapshot) = self.snapshots.get(self.current_index.saturating_sub(1)) {
            // Check lengths match
            let surface_len = surface.data().map(|d| d.len()).unwrap_or(0);
            if surface_len != snapshot.data.len() {
                return false;
            }
            
            // Copy data in separate scope
            {
                let mut surface_data = match surface.data() {
                    Ok(d) => d,
                    Err(_) => return false,
                };
                surface_data.copy_from_slice(&snapshot.data);
            } // surface_data dropped here
            
            surface.mark_dirty();
            return true;
        }
        false
    }

    pub fn undo(&mut self) -> bool {
        if self.current_index > 0 {
            self.current_index -= 1;
            true
        } else {
            false
        }
    }

    pub fn redo(&mut self) -> bool {
        if self.current_index < self.snapshots.len() {
            self.current_index += 1;
            true
        } else {
            false
        }
    }

    pub fn can_undo(&self) -> bool {
        self.current_index > 0
    }

    pub fn can_redo(&self) -> bool {
        self.current_index < self.snapshots.len()
    }

    pub fn clear(&mut self) {
        self.snapshots.clear();
        self.current_index = 0;
    }
    
    pub fn is_at_start(&self) -> bool {
        self.current_index == 0 || self.snapshots.is_empty()
    }
}
