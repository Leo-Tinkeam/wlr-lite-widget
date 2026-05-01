use std::io::BufReader;
use zune_jpeg::{JpegDecoder, zune_core::{colorspace::ColorSpace, options::DecoderOptions}};
use crate::backend_common::surface_common::DrawImageError;

pub(crate) fn render_jpg<F>(mut draw_image: F, path: &str) -> Result<(), DrawImageError>
where
    F: FnMut(Vec<u8>, u32, u32) -> Result<(), DrawImageError>
{
    let file = std::fs::File::open(path)
        .map_err(|_| DrawImageError::LoadImageError)?;
    let options = DecoderOptions::default().jpeg_set_out_colorspace(ColorSpace::BGRA);
    let mut decoder = JpegDecoder::new_with_options(BufReader::new(file), options);
    let pixels = decoder.decode()
        .map_err(|_| DrawImageError::DecodeImageError)?;
    let info = decoder.info()
        .ok_or(DrawImageError::NoMetaDataError)?;
    let img_width = info.width as u32;
    let img_height = info.height as u32;

    draw_image(pixels, img_width, img_height)?;

    Ok(())
}