use wlr_lite_widget::{MouseButton, MouseResponse, SizeUnit, Surface, Widget, WidgetPosition, WidgetSize};
use tiny_skia::{PixmapMut, Color, Paint, Rect, Transform};
use std::thread;
use std::time::Duration;

#[derive(Default, Clone)]
pub struct MyStruct {
    clicked: bool,
}

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
    ).on_press(|my_struct: &mut MyStruct, button| {
        if button == &MouseButton::LEFT {
            my_struct.clicked = true;
            return MouseResponse { do_default: false, need_redraw: true };
        }
        return MouseResponse { do_default: false, need_redraw: false };
    });

    let half_size = WidgetSize {
        width: SizeUnit::Percent(50f32),
        height: SizeUnit::Percent(100f32),
    };
    let surface2 = Surface::new(
        half_size,
        position_0,
        render2
    ).on_press(|my_struct: &mut MyStruct, button| {
        if button == &MouseButton::LEFT {
            my_struct.clicked = false;
            return MouseResponse { do_default: true, need_redraw: true };
        }
        return MouseResponse { do_default: false, need_redraw: false };
    });

    let widget_position = WidgetPosition::Coordinates(SizeUnit::Percent(10f32), SizeUnit::Percent(10f32));

    let mut widget = Widget::new(widget_size, widget_position, "MyWidget".to_string(), None)
        .on_press(|my_struct: &mut MyStruct, button| {
            if button == &MouseButton::LEFT {
                //my_struct.clicked = !my_struct.clicked;
                return MouseResponse { do_default: true, need_redraw: true };
            }
            return MouseResponse { do_default: true, need_redraw: false };
        });

    widget.add_surface(surface);
    widget.add_surface(surface2);

    let thread_widget = widget.clone();
    thread::spawn(move || {
        let mut my_state = MyStruct { clicked: true };
        loop {
            my_state.clicked = !my_state.clicked;
            thread_widget.update_app_state(my_state.clone());
            thread_widget.redraw();
            thread::sleep(Duration::from_secs(1));
            if !thread_widget.is_running() {
                break;
            }
        }
    });

    widget.run();

    println!("--- Fin de l'exemple ---");
}

fn render(canvas: &mut [u8], width: u32, height: u32, app_state: &mut MyStruct) {
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
    let color = if app_state.clicked {
        Color::from_rgba8(255, 0, 0, 255)
    } else {
        Color::from_rgba8(0, 255, 0, 255)
    };

    // Configuration du "pinceau"
    let mut paint = Paint::default();
    paint.set_color(color);
    // paint.anti_alias = true; // Pas utile pour un rectangle droit, mais utile pour des cercles ou formes avancées

    println!("Drawing 1!");
    // 3. Dessin du rectangle
    if let Some(rect) = Rect::from_xywh(0.0, 0.0, width as f32, height as f32) {
        // Transform::identity() veut dire "pas de rotation/zoom"
        // None est pour le clipping mask (masque d'écrêtage)
        pixmap.fill_rect(rect, &paint, Transform::identity(), None);
    }
}

fn render2(canvas: &mut [u8], width: u32, height: u32, app_state: &mut MyStruct) {
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
    let color = if !app_state.clicked {
        Color::from_rgba8(255, 0, 0, 255)
    } else {
        Color::from_rgba8(0, 255, 0, 255)
    };

    // Configuration du "pinceau"
    let mut paint = Paint::default();
    paint.set_color(color);
    // paint.anti_alias = true; // Pas utile pour un rectangle droit, mais utile pour des cercles ou formes avancées

    println!("Drawing 2!");
    // 3. Dessin du rectangle
    if let Some(rect) = Rect::from_xywh(0.0, 0.0, width as f32, height as f32) {
        // Transform::identity() veut dire "pas de rotation/zoom"
        // None est pour le clipping mask (masque d'écrêtage)
        pixmap.fill_rect(rect, &paint, Transform::identity(), None);
    }
}