use wlr_lite_widget::{SizeUnit, Surface, Widget, WidgetPosition, WidgetSize};
use tiny_skia::{PixmapMut, Color, Paint, Rect, Transform};

fn main() {
    println!("--- Début de l'exemple ---");

    let widget_size = WidgetSize {
        width: SizeUnit::Percent(50f32),
        height: SizeUnit::Percent(20f32),
    };

    let full_size = WidgetSize {
        width: SizeUnit::Percent(100f32),
        height: SizeUnit::Percent(100f32),
    };
    let position_0 = WidgetPosition::Coordinates(SizeUnit::Pixel(0), SizeUnit::Pixel(0));

    let surface = Surface::new(
        full_size,
        position_0.clone(),
        render
    );

    let half_size = WidgetSize {
        width: SizeUnit::Percent(50f32),
        height: SizeUnit::Percent(100f32),
    };
    let surface2 = Surface::new(
        half_size,
        position_0,
        render2
    );

    let widget_position = WidgetPosition::Coordinates(SizeUnit::Percent(10f32), SizeUnit::Percent(10f32));

    let mut widget = Widget::new(widget_size, widget_position, "MyWidget".to_string(), None);

    widget.add_surface(surface);
    widget.add_surface(surface2);

    widget.run();

    println!("--- Fin de l'exemple ---");
}

fn render(canvas: &mut [u8], width: u32, height: u32, clicked: bool) { // TODO: Need a custom state to replace "clicked"
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

fn render2(canvas: &mut [u8], width: u32, height: u32, clicked: bool) { // TODO: Need a custom state to replace "clicked"
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
    let color = if !clicked {
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