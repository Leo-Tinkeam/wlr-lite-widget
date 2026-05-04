mod widget;
mod settings;
#[cfg(feature = "cairo-rs")]
mod cairo_surface;
#[cfg(feature = "tiny-skia")]
mod skia_surface;
mod surface;
mod mouse_handler;
mod widget_builder;

mod backend_common;
use backend_common::surface_common;
use backend_common::rectangle;

#[cfg(feature = "cairo-rs")]
pub use cairo_surface::{CairoDrawArea, CairoSurface, WithCairo};
pub use mouse_handler::{MouseButton, MouseResponse};
pub use rectangle::Rectangle;
pub use settings::{SizeUnit, WidgetAnchor, WidgetMargin, WidgetPosition, WidgetSize};
pub use smithay_client_toolkit::seat::pointer::AxisScroll;
pub use smithay_client_toolkit::shell::wlr_layer::{Anchor, Layer};
#[cfg(feature = "tiny-skia")]
pub use skia_surface::{SkiaDrawArea, SkiaSurface, WithSkia};
pub use surface::{CanvasType, no_render, Surface, WithCanvasRender};
pub use surface_common::{load_font, StandardDrawArea, SurfaceBox, SurfaceTrait};
pub use widget_builder::{DrawAreaType, WidgetBuilder};

pub(crate) use mouse_handler::MouseHandler;
pub(crate) use settings::Margin;
pub(crate) use surface_common::{get_next_surface_id, SurfaceData};
pub(crate) use widget::{SharedWidget, Widget, WidgetEvent, WidgetState};
