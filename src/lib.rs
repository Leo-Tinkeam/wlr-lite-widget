mod widget;
mod settings;
#[cfg(feature = "tiny-skia")]
mod skia_surface;
mod surface;
mod surface_common;
mod mouse_handler;
mod widget_builder;

pub use mouse_handler::{MouseButton, MouseResponse};
pub use settings::{SizeUnit, WidgetAnchor, WidgetMargin, WidgetPosition, WidgetSize};
pub use smithay_client_toolkit::seat::pointer::AxisScroll;
pub use smithay_client_toolkit::shell::wlr_layer::{Anchor, Layer};
#[cfg(feature = "tiny-skia")]
pub use skia_surface::{SkiaDrawArea, SkiaSurface, WithSkia};
pub use surface::{Surface, no_render};
pub use surface_common::{SurfaceBox, SurfaceTrait};
pub use widget_builder::{CanvasType, DrawAreaType, WidgetBuilder, WithCanvasRender};

pub(crate) use mouse_handler::MouseHandler;
pub(crate) use settings::Margin;
#[cfg(feature = "tiny-skia")]
pub(crate) use skia_surface::get_skia_draw_area;
pub(crate) use surface_common::{get_next_surface_id, SurfaceData};
pub(crate) use widget::{SharedWidget, Widget, WidgetEvent, WidgetState};
