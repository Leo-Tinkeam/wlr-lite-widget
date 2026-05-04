

pub struct Rectangle {
    pub(crate) x: u32,
    pub(crate) y: u32,
    pub(crate) width: u32,
    pub(crate) height: u32,
    pub(crate) r: u8,
    pub(crate) g: u8,
    pub(crate) b: u8,
    pub(crate) a: u8,
}

impl Rectangle {

    pub fn from_edges(left: u32, top: u32, right: u32, bottom: u32) -> Self {
        Rectangle {
            x: left,
            y: top,
            width: right-left,
            height: bottom-top,
            r: 0,
            g: 0,
            b: 0,
            a: 255,
        }
    }

    pub fn from_coords(x: u32, y: u32, width: u32, height: u32) -> Self {
        Rectangle {
            x,
            y,
            width,
            height,
            r: 0,
            g: 0,
            b: 0,
            a: 255,
        }
    }

    pub fn with_color_rgba(self, r: u8, g: u8, b: u8, a: u8) -> Self {
        Rectangle {
            r,
            g,
            b,
            a,
            ..self
        }
    }

}