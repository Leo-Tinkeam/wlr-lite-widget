use swash::{FontRef, scale::{Render, ScaleContext, Source, StrikeWith, image::Image}, shape::ShapeContext, zeno::{Format, Placement}};
use crate::backend_common::surface_common::DrawTextError;

pub(crate) fn render_text<F>(mut draw_glyph: F,text: &str, font_bytes: &[u8], left: u32, top: u32, text_size: f32) -> Result<(), crate::surface_common::DrawTextError>
where
    F: FnMut(Image, Placement, f32, f32)
{
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
                .format(Format::Alpha)
                .render(&mut scaler, glyph.id);

            if let Some(image) = image {
                let placement = image.placement;
                let img_x = current_x + placement.left as f32 + glyph.x;
                let img_y = y - placement.top as f32 + glyph.y;
                draw_glyph(image, placement, img_x, img_y);
            }
            current_x += glyph.advance;
        }
    });

    Ok(())
}