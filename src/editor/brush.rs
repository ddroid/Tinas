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
}
