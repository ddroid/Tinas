use cairo::Context;

pub struct Brush {
    pub size: f64,
    pub color: (f64, f64, f64, f64), // RGBA
}

impl Brush {
    pub fn new(size: f64, color: (f64, f64, f64, f64)) -> Self {
        Self { size, color }
    }

    pub fn draw(&self, ctx: &Context, x: f64, y: f64) {
        ctx.set_source_rgba(self.color.0, self.color.1, self.color.2, self.color.3);
        
        // Draw a circle for the brush
        ctx.new_path();
        ctx.arc(x, y, self.size / 2.0, 0.0, 2.0 * std::f64::consts::PI);
        ctx.fill().expect("Failed to fill brush stroke");
    }
    
    pub fn draw_line(&self, ctx: &Context, x1: f64, y1: f64, x2: f64, y2: f64) {
        ctx.set_source_rgba(self.color.0, self.color.1, self.color.2, self.color.3);
        ctx.set_line_width(self.size);
        ctx.set_line_cap(cairo::LineCap::Round);
        ctx.set_line_join(cairo::LineJoin::Round);
        
        ctx.new_path();
        ctx.move_to(x1, y1);
        ctx.line_to(x2, y2);
        ctx.stroke().expect("Failed to draw brush line");
    }
}

pub struct Eraser {
    pub size: f64,
}

impl Eraser {
    pub fn new(size: f64) -> Self {
        Self { size }
    }

    pub fn erase(&self, ctx: &Context, x: f64, y: f64) {
        ctx.set_operator(cairo::Operator::Clear);
        ctx.new_path();
        ctx.arc(x, y, self.size / 2.0, 0.0, 2.0 * std::f64::consts::PI);
        ctx.fill().expect("Failed to erase");
        ctx.set_operator(cairo::Operator::Over);
    }
    
    pub fn erase_line(&self, ctx: &Context, x1: f64, y1: f64, x2: f64, y2: f64) {
        ctx.set_operator(cairo::Operator::Clear);
        ctx.set_line_width(self.size);
        ctx.set_line_cap(cairo::LineCap::Round);
        ctx.set_line_join(cairo::LineJoin::Round);
        
        ctx.new_path();
        ctx.move_to(x1, y1);
        ctx.line_to(x2, y2);
        ctx.stroke().expect("Failed to erase line");
        ctx.set_operator(cairo::Operator::Over);
    }
}
