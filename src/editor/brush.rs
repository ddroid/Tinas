use cairo::Context;
use std::fmt;

/// Brush shape types
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum BrushShape {
    Circle,
    Square,
    Line,
}

impl fmt::Display for BrushShape {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BrushShape::Circle => write!(f, "Circle"),
            BrushShape::Square => write!(f, "Square"),
            BrushShape::Line => write!(f, "Line"),
        }
    }
}

/// Brush with configurable shape, size, color, and tracking
pub struct Brush {
    pub size: f64,
    pub color: (f64, f64, f64, f64), // RGBA
    pub shape: BrushShape,
    pub hardness: f64, // 0.0 = soft edge, 1.0 = hard edge
}

impl Brush {
    pub fn new(size: f64, color: (f64, f64, f64, f64)) -> Self {
        Self {
            size,
            color,
            shape: BrushShape::Circle,
            hardness: 1.0,
        }
    }

    pub fn with_shape(mut self, shape: BrushShape) -> Self {
        self.shape = shape;
        self
    }

    pub fn with_hardness(mut self, hardness: f64) -> Self {
        self.hardness = hardness.clamp(0.0, 1.0);
        self
    }

    pub fn set_size(&mut self, size: f64) {
        self.size = size.max(1.0);
    }

    pub fn set_color(&mut self, r: f64, g: f64, b: f64, a: f64) {
        self.color = (r.clamp(0.0, 1.0), g.clamp(0.0, 1.0), b.clamp(0.0, 1.0), a.clamp(0.0, 1.0));
    }

    pub fn set_shape(&mut self, shape: BrushShape) {
        self.shape = shape;
    }

    pub fn set_hardness(&mut self, hardness: f64) {
        self.hardness = hardness.clamp(0.0, 1.0);
    }

    /// Get brush info for tracking/debugging
    pub fn get_info(&self) -> String {
        format!(
            "Brush: {}px {} RGBA({:.2}, {:.2}, {:.2}, {:.2})",
            self.size, self.shape, self.color.0, self.color.1, self.color.2, self.color.3
        )
    }

    pub fn draw(&self, ctx: &Context, x: f64, y: f64) {
        ctx.set_source_rgba(self.color.0, self.color.1, self.color.2, self.color.3);
        
        match self.shape {
            BrushShape::Circle => {
                ctx.new_path();
                ctx.arc(x, y, self.size / 2.0, 0.0, 2.0 * std::f64::consts::PI);
                ctx.fill().expect("Failed to fill brush stroke");
            }
            BrushShape::Square => {
                let half = self.size / 2.0;
                ctx.new_path();
                ctx.rectangle(x - half, y - half, self.size, self.size);
                ctx.fill().expect("Failed to fill square brush");
            }
            BrushShape::Line => {
                // For line shape, draw a small vertical line segment
                ctx.set_line_width(self.size);
                ctx.set_line_cap(cairo::LineCap::Round);
                ctx.new_path();
                ctx.move_to(x, y - self.size / 2.0);
                ctx.line_to(x, y + self.size / 2.0);
                ctx.stroke().expect("Failed to draw line brush");
            }
        }
    }
    
    pub fn draw_line(&self, ctx: &Context, x1: f64, y1: f64, x2: f64, y2: f64) {
        ctx.set_source_rgba(self.color.0, self.color.1, self.color.2, self.color.3);
        
        match self.shape {
            BrushShape::Circle => {
                ctx.set_line_width(self.size);
                ctx.set_line_cap(cairo::LineCap::Round);
                ctx.set_line_join(cairo::LineJoin::Round);
                
                ctx.new_path();
                ctx.move_to(x1, y1);
                ctx.line_to(x2, y2);
                ctx.stroke().expect("Failed to draw brush line");
            }
            BrushShape::Square => {
                // For square brush, draw a rectangle along the line
                let dx = x2 - x1;
                let dy = y2 - y1;
                let angle = dy.atan2(dx);
                let half_size = self.size / 2.0;
                
                // Calculate perpendicular offset
                let perp_x = -angle.sin() * half_size;
                let perp_y = angle.cos() * half_size;
                
                ctx.new_path();
                ctx.move_to(x1 + perp_x, y1 + perp_y);
                ctx.line_to(x2 + perp_x, y2 + perp_y);
                ctx.line_to(x2 - perp_x, y2 - perp_y);
                ctx.line_to(x1 - perp_x, y1 - perp_y);
                ctx.close_path();
                ctx.fill().expect("Failed to fill square brush line");
            }
            BrushShape::Line => {
                // Line brush just draws a thicker line
                ctx.set_line_width(self.size);
                ctx.set_line_cap(cairo::LineCap::Round);
                ctx.new_path();
                ctx.move_to(x1, y1);
                ctx.line_to(x2, y2);
                ctx.stroke().expect("Failed to draw line brush stroke");
            }
        }
    }
}

pub struct Eraser {
    pub size: f64,
    pub shape: BrushShape,
}

impl Eraser {
    pub fn new(size: f64) -> Self {
        Self {
            size,
            shape: BrushShape::Circle,
        }
    }

    pub fn set_size(&mut self, size: f64) {
        self.size = size.max(1.0);
    }

    pub fn set_shape(&mut self, shape: BrushShape) {
        self.shape = shape;
    }

    pub fn erase(&self, ctx: &Context, x: f64, y: f64) {
        ctx.set_operator(cairo::Operator::Clear);
        
        match self.shape {
            BrushShape::Circle => {
                ctx.new_path();
                ctx.arc(x, y, self.size / 2.0, 0.0, 2.0 * std::f64::consts::PI);
                ctx.fill().expect("Failed to erase");
            }
            BrushShape::Square => {
                let half = self.size / 2.0;
                ctx.new_path();
                ctx.rectangle(x - half, y - half, self.size, self.size);
                ctx.fill().expect("Failed to erase square");
            }
            BrushShape::Line => {
                ctx.set_line_width(self.size);
                ctx.set_line_cap(cairo::LineCap::Round);
                ctx.new_path();
                ctx.move_to(x, y - self.size / 2.0);
                ctx.line_to(x, y + self.size / 2.0);
                ctx.stroke().expect("Failed to erase line");
            }
        }
        
        ctx.set_operator(cairo::Operator::Over);
    }
    
    pub fn erase_line(&self, ctx: &Context, x1: f64, y1: f64, x2: f64, y2: f64) {
        ctx.set_operator(cairo::Operator::Clear);
        
        match self.shape {
            BrushShape::Circle => {
                ctx.set_line_width(self.size);
                ctx.set_line_cap(cairo::LineCap::Round);
                ctx.set_line_join(cairo::LineJoin::Round);
                
                ctx.new_path();
                ctx.move_to(x1, y1);
                ctx.line_to(x2, y2);
                ctx.stroke().expect("Failed to erase line");
            }
            BrushShape::Square => {
                let dx = x2 - x1;
                let dy = y2 - y1;
                let angle = dy.atan2(dx);
                let half_size = self.size / 2.0;
                
                let perp_x = -angle.sin() * half_size;
                let perp_y = angle.cos() * half_size;
                
                ctx.new_path();
                ctx.move_to(x1 + perp_x, y1 + perp_y);
                ctx.line_to(x2 + perp_x, y2 + perp_y);
                ctx.line_to(x2 - perp_x, y2 - perp_y);
                ctx.line_to(x1 - perp_x, y1 - perp_y);
                ctx.close_path();
                ctx.fill().expect("Failed to fill erase");
            }
            BrushShape::Line => {
                ctx.set_line_width(self.size);
                ctx.set_line_cap(cairo::LineCap::Round);
                ctx.new_path();
                ctx.move_to(x1, y1);
                ctx.line_to(x2, y2);
                ctx.stroke().expect("Failed to erase line stroke");
            }
        }
        
        ctx.set_operator(cairo::Operator::Over);
    }
}

/// Brush stroke data for tracking and history
#[derive(Clone, Debug)]
pub struct BrushStroke {
    pub points: Vec<(f64, f64)>,
    pub tool_type: ToolType,
    pub size: f64,
    pub color: (f64, f64, f64, f64),
    pub shape: BrushShape,
    pub timestamp: std::time::Instant,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ToolType {
    Brush,
    Eraser,
}

impl BrushStroke {
    pub fn new(tool_type: ToolType, size: f64, color: (f64, f64, f64, f64), shape: BrushShape) -> Self {
        Self {
            points: Vec::new(),
            tool_type,
            size,
            color,
            shape,
            timestamp: std::time::Instant::now(),
        }
    }

    pub fn add_point(&mut self, x: f64, y: f64) {
        self.points.push((x, y));
    }

    pub fn replay(&self, ctx: &Context) {
        if self.points.is_empty() {
            return;
        }

        match self.tool_type {
            ToolType::Brush => {
                let brush = Brush::new(self.size, self.color).with_shape(self.shape);
                
                // Draw first point
                brush.draw(ctx, self.points[0].0, self.points[0].1);
                
                // Draw lines between points
                for i in 1..self.points.len() {
                    brush.draw_line(ctx, self.points[i-1].0, self.points[i-1].1, 
                                         self.points[i].0, self.points[i].1);
                }
            }
            ToolType::Eraser => {
                let eraser = Eraser::new(self.size);
                
                // Erase first point
                eraser.erase(ctx, self.points[0].0, self.points[0].1);
                
                // Erase between points
                for i in 1..self.points.len() {
                    eraser.erase_line(ctx, self.points[i-1].0, self.points[i-1].1,
                                           self.points[i].0, self.points[i].1);
                }
            }
        }
    }
}
