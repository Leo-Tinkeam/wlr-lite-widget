use wlr_lite_widget::{Anchor, SizeUnit, Widget, WidgetDetails, WidgetSize};
use tiny_skia::{PixmapMut, Color, Paint, Rect, Transform};

fn main() {
    println!("--- Début de l'exemple ---");

    let widget_size = WidgetSize {
        width: SizeUnit::Percent(50f32),
        height: SizeUnit::Percent(20f32),
    };

    let widget_details = WidgetDetails {
        anchor: Some(Anchor::TOP),
        ..Default::default()
    };

    Widget::new(widget_size, "MyWidget".to_string(), Some(widget_details), render);

    println!("--- Fin de l'exemple ---");
}

fn render(canvas: &mut [u8], width: u32, height: u32, clicked: bool) {
    let ptr = canvas.as_mut_ptr();
    let len = canvas.len();
    let fake_canvas = unsafe { std::slice::from_raw_parts_mut(ptr, len) };

    // 1. Création de la Pixmap "wrapper"
    let mut pixmap = PixmapMut::from_bytes(
        fake_canvas, 
        width,
        height,
    ).expect("Erreur taille buffer / stride");

    // Choix de la couleur
    /* let color = if self.clicked {
        Color::from_rgba8(255, 0, 0, 255)
    } else {
        Color::from_rgba8(0, 255, 0, 255)
    }; */
    let color = if clicked {
        Color::from_rgba8(255, 0, 0, 255)
    } else {
        Color::from_rgba8(0, 255, 0, 255)
    };

    // Configuration du "pinceau"
    let mut paint = Paint::default();
    paint.set_color(color);
    // paint.anti_alias = true; // Pas utile pour un rectangle droit, mais utile pour des cercles ou formes avancées

    println!("Drawing!");
    // 3. Dessin du rectangle
    if let Some(rect) = Rect::from_xywh(0.0, 0.0, width as f32, height as f32) {
        // Transform::identity() veut dire "pas de rotation/zoom"
        // None est pour le clipping mask (masque d'écrêtage)
        pixmap.fill_rect(rect, &paint, Transform::identity(), None);
    }
}