use std::marker::PhantomData;
use cairo::{Context, Format, ImageSurface};
use swash::{FontRef, scale::{Render, ScaleContext, Source, StrikeWith, image::Content}, shape::ShapeContext, zeno::Format as FormatZeno};
use crate::{DrawAreaType, MouseHandler, StandardDrawArea, SurfaceBox, SurfaceData, SurfaceTrait, WidgetPosition, WidgetSize, get_next_surface_id, surface_common::DrawTextError};

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
        let (r, g, b, a) = (r as f32 / 255.0, g as f32 / 255.0, b as f32 / 255.0, a as f32 / 255.0);
        let font = FontRef::from_index(font_bytes, 0)
            .ok_or(DrawTextError::LoadFileError)?;
        let mut shape_context = ShapeContext::new();
        let mut shaper = shape_context
            .builder(font)
            .size(text_size)
            .build();
        shaper.add_str(text);
        let mut scale_context = ScaleContext::new();
        let mut scaler = scale_context
            .builder(font)
            .size(text_size)
            .hint(true)
            .build();

        let mut current_x = left as f32;
        let y = top as f32 + text_size;
        shaper.shape_with(|cluster| {
            for glyph in cluster.glyphs {
                let image = Render::new(&[
                        Source::ColorOutline(0),
                        Source::ColorBitmap(StrikeWith::BestFit),
                        Source::Outline])
                    .format(FormatZeno::Alpha)
                    .render(&mut scaler, glyph.id);

                if let Some(image) = image {
                    let placement = image.placement;
                    let img_x = current_x + placement.left as f32 + glyph.x;
                    let img_y = y - placement.top as f32 + glyph.y;

                    if let Ok(mut surface) = cairo::ImageSurface::create(
                        cairo::Format::ARgb32,
                        placement.width as i32,
                        placement.height as i32,
                    ) {
                        if let Ok(mut data) = surface.data() {
                            match image.content {
                                Content::Mask => {
                                    for (out_pixel, &alpha_byte) in data.chunks_exact_mut(4).zip(image.data.iter()) {
                                        let final_alpha = alpha_byte as f32 * a;
                                        let a_byte = (final_alpha) as u32;
                                        let r_byte = (r * final_alpha) as u32;
                                        let g_byte = (g * final_alpha) as u32;
                                        let b_byte = (b * final_alpha) as u32;
                                        let pixel_u32 = (a_byte << 24) | (r_byte << 16) | (g_byte << 8) | b_byte;
                                        out_pixel.copy_from_slice(&pixel_u32.to_ne_bytes());
                                    }
                                },
                                Content::Color => {
                                    for (out_pixel, in_pixel) in data.chunks_exact_mut(4).zip(image.data.chunks_exact(4)) {
                                        let final_alpha = in_pixel[3] as f32 * a / 255.0;
                                        let a_byte = (final_alpha * 255.0) as u32;
                                        let r_byte = (in_pixel[0] as f32 * final_alpha) as u32;
                                        let g_byte = (in_pixel[1] as f32 * final_alpha) as u32;
                                        let b_byte = (in_pixel[2] as f32 * final_alpha) as u32;
                                        let pixel_u32 = (a_byte << 24) | (r_byte << 16) | (g_byte << 8) | b_byte;
                                        out_pixel.copy_from_slice(&pixel_u32.to_ne_bytes());
                                    }
                                },
                                Content::SubpixelMask => {
                                    // Should not be called since we are using Format::Alpha, we just support Content:Color for emojis
                                }
                            }
                        } 
                        surface.mark_dirty();
                        self.context.set_source_surface(&surface, img_x as f64, img_y as f64).unwrap();
                        self.context.paint().unwrap();
                    }
                }
                current_x += glyph.advance;
            }
        });


        Ok(())
    }
}