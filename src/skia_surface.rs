use std::marker::PhantomData;
use tiny_skia::{Paint, PixmapMut, Rect, Transform};
use crate::{MouseHandler, SurfaceBox, SurfaceData, SurfaceTrait, WidgetPosition, WidgetSize, get_next_surface_id, widget_builder::DrawAreaType};

pub struct WithSkia;

impl DrawAreaType for WithSkia {
    type Type<'a> = SkiaDrawArea<'a>;
}

pub struct SkiaSurface<T> {
    pub(crate) render: Box<dyn for<'a> FnMut(&mut SkiaDrawArea, u32, u32, SurfaceBox, &mut T) + Send>,
    surface_data: SurfaceData<T, WithSkia, Self>,
}

pub(crate) fn get_skia_draw_area<'a>(canvas: &'a mut [u8], width: u32, height: u32) -> SkiaDrawArea<'a> {
    SkiaDrawArea {
        pixmap: PixmapMut::from_bytes(
            canvas, 
            width,
            height,
        ).expect("Erreur taille buffer / stride")
    }
}

impl<T> SurfaceTrait<T, WithSkia> for SkiaSurface<T> {
    fn get_surface_data_mut(&mut self) -> &mut SurfaceData<T, WithSkia, SkiaSurface<T>> {
        &mut self.surface_data
    }

    fn get_surface_data(&self) -> &SurfaceData<T, WithSkia, SkiaSurface<T>> {
        &self.surface_data
    }

    fn render<'a>(&mut self, draw_area: &mut <WithSkia as DrawAreaType>::Type<'a>, width: u32, height: u32, surface_box: SurfaceBox, app_state: &mut T) {
        (self.render)(draw_area, width, height, surface_box, app_state);
    }
}

impl<T: Default> SkiaSurface<T> {
    pub fn new<F>(size: WidgetSize, position: WidgetPosition, render: F) -> Self
    where
        F: for<'a> FnMut(&mut SkiaDrawArea, u32, u32, SurfaceBox, &mut T) + 'static + Send
    {
        let id = get_next_surface_id();

        SkiaSurface {
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

pub struct SkiaDrawArea<'a> {
    pixmap: PixmapMut<'a>,
}

impl<'a> SkiaDrawArea<'a> {
    pub fn add_rect(&mut self, left: u32, top: u32, right: u32, bottom: u32, r: u8, g: u8, b: u8, a: u8) {
        let mut paint = Paint::default();
        let rect = Rect::from_ltrb(
            left as f32,
            top as f32,
            right as f32,
            bottom as f32,
        ).unwrap();
        paint.set_color_rgba8(b, g, r, a); // Wayland buffer are BGRA
        self.pixmap.fill_rect(
            rect,
            &paint,
            Transform::identity(),
            None,
        );
    }
}
