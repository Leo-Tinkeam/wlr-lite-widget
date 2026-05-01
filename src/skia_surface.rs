use std::marker::PhantomData;
use swash::scale::image::Content;
use tiny_skia::{Color, Paint, Pixmap, PixmapMut, PixmapPaint, Rect, Transform};
use crate::{MouseHandler, StandardDrawArea, SurfaceBox, SurfaceData, SurfaceTrait, WidgetPosition, WidgetSize, backend_common::text_shaper::render_text, get_next_surface_id, surface_common::DrawTextError, widget_builder::DrawAreaType};

pub struct WithSkia;

impl DrawAreaType for WithSkia {
    type Type<'a> = SkiaDrawArea<'a>;

    fn get_draw_area<'a>(canvas: &'a mut [u8], width: u32, height: u32) -> Self::Type<'a> {
        SkiaDrawArea {
            pixmap: PixmapMut::from_bytes(
                canvas, 
                width,
                height,
            ).expect("Erreur taille buffer / stride")
        }
    }
}

pub struct SkiaSurface<T> {
    pub(crate) render: Box<dyn for<'a> FnMut(&mut SkiaDrawArea, u32, u32, SurfaceBox, &mut T) + Send>,
    surface_data: SurfaceData<T, WithSkia, Self>,
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

impl<'a> StandardDrawArea for SkiaDrawArea<'a> {
    fn add_rect(&mut self, left: u32, top: u32, right: u32, bottom: u32, r: u8, g: u8, b: u8, a: u8) {
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

    fn add_text(&mut self, text: &str, font_bytes: &[u8], left: u32, top: u32, text_size: f32, r: u8, g: u8, b: u8, a: u8) -> Result<(), DrawTextError> {
        let text_color = Color::from_rgba8(r, g, b, a);
        render_text(|image, placement, img_x, img_y|
            {
                if let Some(mut glyph_pixmap) = Pixmap::new(placement.width, placement.height) {
                    match image.content {
                        Content::Mask => {
                            for (i, pixel) in glyph_pixmap.pixels_mut().iter_mut().enumerate() {
                                let alpha = image.data[i] as f32 / 255.0;
                                let c = Color::from_rgba( // Wayland is BGRA
                                    text_color.blue(),
                                    text_color.green(),
                                    text_color.red(),
                                    text_color.alpha() * alpha,
                                ).unwrap_or(Color::TRANSPARENT);
                                *pixel = c.premultiply().to_color_u8();
                            }
                        },
                        Content::Color => {
                            for (i, pixel) in glyph_pixmap.pixels_mut().iter_mut().enumerate() {
                                let c = Color::from_rgba( // Wayland is BGRA
                                    image.data[i * 4 + 2] as f32 / 255.0,
                                    image.data[i * 4 + 1] as f32 / 255.0,
                                    image.data[i * 4] as f32 / 255.0,
                                    (image.data[i * 4 + 3] as f32 / 255.0) * text_color.alpha(),
                                ).unwrap_or(Color::TRANSPARENT);
                                *pixel = c.premultiply().to_color_u8();
                            }
                        },
                        Content::SubpixelMask => {
                            // Should not be called since we are using Format::Alpha, we just support Content:Color for emojis
                        }
                    }
                    
                    self.pixmap.draw_pixmap(
                        img_x as i32,
                        img_y as i32,
                        glyph_pixmap.as_ref(),
                        &PixmapPaint::default(),
                        Transform::identity(),
                        None,
                    );
                }
            },
            text,
            font_bytes,
            left,
            top,
            text_size,
        )
    }
}
