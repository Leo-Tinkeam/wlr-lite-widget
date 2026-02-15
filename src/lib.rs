mod layer;
mod settings;

pub use layer::Widget;
pub use settings::{SizeUnit, WidgetAnchor, WidgetMargin, WidgetPosition, WidgetSize};
pub use smithay_client_toolkit::shell::wlr_layer::{Anchor, Layer};

pub(crate) use settings::Margin;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        assert_eq!(4, 4);
    }
}
