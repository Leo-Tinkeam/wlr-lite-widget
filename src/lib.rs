mod layer;

pub use layer::Layer;

pub enum SizeUnit {
    Percent(f32),
    Pixel(u32),
}

pub struct WidgetSize {
    pub width: SizeUnit,
    pub height: SizeUnit,
}

impl WidgetSize {

    pub fn get_dimension(&mut self, screen_width: u32, screen_height: u32) -> (Option<u32>, Option<u32>) {
        let width = match self.width {
            SizeUnit::Percent(percent) => ((screen_width as f32)*percent/100f32) as u32,
            SizeUnit::Pixel(pixel) => pixel,
        };
        let height = match self.height {
            SizeUnit::Percent(percent) => ((screen_height as f32)*percent/100f32) as u32,
            SizeUnit::Pixel(pixel) => pixel,
        };
        (Some(width), Some(height))
    }

}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        assert_eq!(4, 4);
    }
}
