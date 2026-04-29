use std::marker::PhantomData;
use cairo::{Context, Format, ImageSurface};
use crate::{DrawAreaType, MouseHandler, StandardDrawArea, SurfaceBox, SurfaceData, SurfaceTrait, WidgetPosition, WidgetSize, get_next_surface_id};

pub struct WithCairo;

impl DrawAreaType for WithCairo {
    type Type<'a> = CairoDrawArea<'a>;

    fn get_draw_area<'a>(canvas: &'a mut [u8], width: u32, height: u32) -> Self::Type<'a> {
        let stride = width as i32 * 4;
        // We have a lifetime <'a> on CairoDrawArea that is not required by compiler, it's for this unsafe block
        let surface = unsafe {
            ImageSurface::create_for_data_unsafe(
                canvas.as_mut_ptr(),
                Format::ARgb32,
                width as i32,
                height as i32,
                stride,
            )
        }.expect("Cairo context creation error");
        let context = Context::new(&surface).expect("Cairo context creation error");
        CairoDrawArea {
            context,
            _marker: PhantomData,
        }

    }
}

pub struct CairoSurface<T> {
    pub(crate) render: Box<dyn for<'a> FnMut(&mut CairoDrawArea, u32, u32, SurfaceBox, &mut T) + Send>,
    surface_data: SurfaceData<T, WithCairo, Self>,
}

impl<T> SurfaceTrait<T, WithCairo> for CairoSurface<T> {
    fn get_surface_data_mut(&mut self) -> &mut SurfaceData<T, WithCairo, CairoSurface<T>> {
        &mut self.surface_data
    }

    fn get_surface_data(&self) -> &SurfaceData<T, WithCairo, CairoSurface<T>> {
        &self.surface_data
    }

    fn render<'a>(&mut self, draw_area: &mut <WithCairo as DrawAreaType>::Type<'a>, width: u32, height: u32, surface_box: SurfaceBox, app_state: &mut T) {
        (self.render)(draw_area, width, height, surface_box, app_state);
    }
}

impl<T: Default> CairoSurface<T> {
    pub fn new<F>(size: WidgetSize, position: WidgetPosition, render: F) -> Self
    where
        F: for<'a> FnMut(&mut CairoDrawArea, u32, u32, SurfaceBox, &mut T) + 'static + Send
    {
        let id = get_next_surface_id();

        CairoSurface {
            render: Box::new(render),
            surface_data: SurfaceData {
                id,
                size,
                position,
                need_redraw: true,
                event_sender: None,
                mouse_handler: MouseHandler::default(),
                real_size: None,
                childs_surfaces: vec![],
                _marker: PhantomData,
            },
        }
    }

    // We should be able to animate this on "hover" (maybe render with a 0-1 float)
}

pub struct CairoDrawArea<'a> {
    context: Context,
    // This lifetime comes from the unsafe block in WithCairo::get_draw_area
    _marker: PhantomData<&'a mut [u8]>,
}

impl<'a> StandardDrawArea for CairoDrawArea<'a> {
    fn add_rect(&mut self, left: u32, top: u32, right: u32, bottom: u32, r: u8, g: u8, b: u8, a: u8) {
        let width = (right - left) as f64;
        let height = (bottom - top) as f64;
        self.context.rectangle(
            left as f64,
            top as f64,
            width,
            height,
        );
        self.context.set_source_rgba(
            r as f64 / 255.0,
            g as f64 / 255.0,
            b as f64 / 255.0,
            a as f64 / 255.0,
        );
        self.context.fill().expect("Cairo draw error");
    }

    fn add_text(&mut self, text: &str, font_bytes: &[u8], left: u32, top: u32, text_size: f32, r: u8, g: u8, b: u8, a: u8) -> Result<(), crate::surface_common::DrawTextError> {
        panic!(); // TODO
    }
}