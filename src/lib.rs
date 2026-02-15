mod widget;
mod settings;

pub use widget::Widget;
pub use settings::{SizeUnit, WidgetAnchor, WidgetMargin, WidgetPosition, WidgetSize};
pub use smithay_client_toolkit::shell::wlr_layer::{Anchor, Layer};

pub(crate) use settings::Margin;
