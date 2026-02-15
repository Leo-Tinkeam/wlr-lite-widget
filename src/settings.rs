use smithay_client_toolkit::shell::wlr_layer::Anchor;

pub enum WidgetPosition {
    Coordinates(SizeUnit, SizeUnit),
    Anchor(WidgetAnchor, Option<WidgetMargin>),
}

#[derive(Clone)]
pub enum WidgetAnchor {
    Center,
    Top,
    TopRight,
    Right,
    BottomRight,
    Bottom,
    BottomLeft,
    Left,
    TopLeft,
}

impl From<WidgetAnchor> for Option<Anchor> {
    fn from(widget_anchor: WidgetAnchor) -> Self {
        match widget_anchor {
            WidgetAnchor::Center => None,
            WidgetAnchor::Top => Some(Anchor::TOP),
            WidgetAnchor::TopRight => Some(Anchor::TOP | Anchor::RIGHT),
            WidgetAnchor::Right => Some(Anchor::RIGHT),
            WidgetAnchor::BottomRight => Some(Anchor::BOTTOM | Anchor::RIGHT),
            WidgetAnchor::Bottom => Some(Anchor::BOTTOM),
            WidgetAnchor::BottomLeft => Some(Anchor::BOTTOM | Anchor::LEFT),
            WidgetAnchor::Left => Some(Anchor::LEFT),
            WidgetAnchor::TopLeft => Some(Anchor::TOP | Anchor::LEFT),
        }
    }
}

pub(crate) struct Margin {
    pub(crate) top: SizeUnit,
    pub(crate) right: SizeUnit,
    pub(crate) bottom: SizeUnit,
    pub(crate) left: SizeUnit,
}

impl Margin {
    pub(crate) fn get_margin(&mut self, screen_width: u32, screen_height: u32) -> (i32, i32, i32, i32) {
        let top = match self.top {
            SizeUnit::Percent(percent) => ((screen_height as f32)*percent/100f32) as i32,
            SizeUnit::Pixel(pixel) => pixel as i32,
        };
        let right = match self.right {
            SizeUnit::Percent(percent) => ((screen_width as f32)*percent/100f32) as i32,
            SizeUnit::Pixel(pixel) => pixel as i32,
        };
        let bottom = match self.bottom {
            SizeUnit::Percent(percent) => ((screen_height as f32)*percent/100f32) as i32,
            SizeUnit::Pixel(pixel) => pixel as i32,
        };
        let left = match self.left {
            SizeUnit::Percent(percent) => ((screen_width as f32)*percent/100f32) as i32,
            SizeUnit::Pixel(pixel) => pixel as i32,
        };
        (top, right, bottom, left)
    }
}

#[derive(Default, Clone)]
pub struct WidgetMargin {
    pub top: Option<SizeUnit>,
    pub right: Option<SizeUnit>,
    pub bottom: Option<SizeUnit>,
    pub left: Option<SizeUnit>,
}

impl WidgetMargin {
    pub(crate) fn into_margin(&mut self, anchor: WidgetAnchor) -> Margin {
        let widget_margin = self.clone();
        let top = widget_margin.top.unwrap_or(SizeUnit::Pixel(0));
        if (top != SizeUnit::Pixel(0)) & (top != SizeUnit::Percent(0f32)) {
            match anchor {
                WidgetAnchor::Top | WidgetAnchor::TopLeft | WidgetAnchor::TopRight => (),
                _ => panic!("top margin can not be use without Top anchor"),
            }
        }
        let right = widget_margin.right.unwrap_or(SizeUnit::Pixel(0));
        if (right != SizeUnit::Pixel(0)) & (right != SizeUnit::Percent(0f32)) {
            match anchor {
                WidgetAnchor::Right | WidgetAnchor::TopRight | WidgetAnchor::BottomRight => (),
                _ => panic!("right margin can not be use without Right anchor"),
            }
        }
        let bottom = widget_margin.bottom.unwrap_or(SizeUnit::Pixel(0));
        if (bottom != SizeUnit::Pixel(0)) & (bottom != SizeUnit::Percent(0f32)) {
            match anchor {
                WidgetAnchor::Bottom | WidgetAnchor::BottomLeft | WidgetAnchor::BottomRight => (),
                _ => panic!("bottom margin can not be use without Bottom anchor"),
            }
        }
        let left = widget_margin.left.unwrap_or(SizeUnit::Pixel(0));
        if (left != SizeUnit::Pixel(0)) & (left != SizeUnit::Percent(0f32)) {
            match anchor {
                WidgetAnchor::Left | WidgetAnchor::TopLeft | WidgetAnchor::BottomLeft => (),
                _ => panic!("left margin can not be use without Left anchor"),
            }
        }
        Margin { top, right, bottom, left }
    }
}

#[derive(PartialEq, Clone)]
pub enum SizeUnit {
    Percent(f32),
    Pixel(u32),
}

pub struct WidgetSize {
    pub width: SizeUnit,
    pub height: SizeUnit,
}

impl WidgetSize {
    pub(crate) fn get_dimension(&mut self, screen_width: u32, screen_height: u32) -> (Option<u32>, Option<u32>) {
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