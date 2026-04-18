use wlr_lite_widget::{CanvasType, MouseButton, MouseResponse, SizeUnit, Surface, SurfaceBox, SurfaceTrait, WidgetBuilder, WidgetPosition, WidgetSize, WithCanvasRender, no_render};
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

    let half_size = WidgetSize {
        width: SizeUnit::Percent(50f32),
        height: SizeUnit::Percent(100f32),
    };
    let position_0 = WidgetPosition::Coordinates(SizeUnit::Pixel(0), SizeUnit::Pixel(0));

    let surface = Surface::new(
        half_size.clone(),
        position_0.clone(),
        render
    ).on_press(|my_struct: &mut MyStruct, button| {
        if button == &MouseButton::LEFT {
            my_struct.clicked = true;
            return MouseResponse { do_default: false, need_redraw: true };
        }
        return MouseResponse { do_default: false, need_redraw: false };
    });

    let position_1 = WidgetPosition::Coordinates(SizeUnit::Percent(50f32), SizeUnit::Pixel(0));
    let mut surface2 = Surface::new(
        half_size.clone(),
        position_1.clone(),
        no_render::<MyStruct, WithCanvasRender>
    );

    let surface3 = Surface::new(
        half_size.clone(),
        position_0,
        render2
    ).on_press(|my_struct: &mut MyStruct, button| {
        if button == &MouseButton::LEFT {
            my_struct.clicked = false;
            return MouseResponse { do_default: true, need_redraw: true };
        }
        return MouseResponse { do_default: false, need_redraw: false };
    });
    let surface4 = Surface::new(
        half_size,
        position_1,
        render3
    ).on_press(|my_struct: &mut MyStruct, button| {
        if button == &MouseButton::LEFT {
            my_struct.clicked = true;
            return MouseResponse { do_default: true, need_redraw: true };
        }
        return MouseResponse { do_default: false, need_redraw: false };
    });

    surface2.add_surface(surface3);
    surface2.add_surface(surface4);

    let widget_position = WidgetPosition::Coordinates(SizeUnit::Percent(10f32), SizeUnit::Percent(10f32));

    let mut widget = WidgetBuilder::new(widget_size, widget_position, "MyWidget".to_string())
        .build();

    widget.add_surface(surface);
    widget.add_surface(surface2);

    let thread_widget = widget.clone();
    thread::spawn(move || {
        let mut my_state = MyStruct { clicked: true };
        loop {
            my_state.clicked = !my_state.clicked;
            thread_widget.update_app_state(my_state.clone());
            thread_widget.force_redraw();
            thread::sleep(Duration::from_secs(10));
            if !thread_widget.is_running() {
                break;
            }
        }
    });

    widget.run();

    println!("--- Fin de l'exemple ---");
}

fn render(canvas_struct: &mut CanvasType, widget_width: u32, widget_height: u32, surface_box: SurfaceBox, app_state: &mut MyStruct) {
    let mut pixmap = PixmapMut::from_bytes(
        canvas_struct.canvas, 
        widget_width,
        widget_height,
    ).expect("Erreur taille buffer / stride");
    let (x, y, w, h) = surface_box.get_xywh();

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
    // Dessin du rectangle
    if let Some(rect) = Rect::from_xywh(x, y, w, h) {
        // Transform::identity() veut dire "pas de rotation/zoom"
        // None est pour le clipping mask (masque d'écrêtage)
        pixmap.fill_rect(rect, &paint, Transform::identity(), None);
    }
}

fn render2(canvas_struct: &mut CanvasType, widget_width: u32, widget_height: u32, surface_box: SurfaceBox, app_state: &mut MyStruct) {
    let mut pixmap = PixmapMut::from_bytes(
        canvas_struct.canvas, 
        widget_width,
        widget_height,
    ).expect("Erreur taille buffer / stride");
    let (x, y, w, h) = surface_box.get_xywh();

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
    // Dessin du rectangle
    if let Some(rect) = Rect::from_xywh(x, y, w, h) {
        // Transform::identity() veut dire "pas de rotation/zoom"
        // None est pour le clipping mask (masque d'écrêtage)
        pixmap.fill_rect(rect, &paint, Transform::identity(), None);
    }
}

fn render3(canvas_struct: &mut CanvasType, widget_width: u32, widget_height: u32, surface_box: SurfaceBox, app_state: &mut MyStruct) {
    let mut pixmap = PixmapMut::from_bytes(
        canvas_struct.canvas, 
        widget_width,
        widget_height,
    ).expect("Erreur taille buffer / stride");
    let (x, y, w, h) = surface_box.get_xywh();

    // Choix de la couleur
    let color = if !app_state.clicked {
        Color::from_rgba8(0, 255, 0, 255)
    } else {
        Color::from_rgba8(0, 0, 255, 255)
    };

    // Configuration du "pinceau"
    let mut paint = Paint::default();
    paint.set_color(color);
    // paint.anti_alias = true; // Pas utile pour un rectangle droit, mais utile pour des cercles ou formes avancées

    println!("Drawing 2!");
    // Dessin du rectangle
    if let Some(rect) = Rect::from_xywh(x, y, w, h) {
        // Transform::identity() veut dire "pas de rotation/zoom"
        // None est pour le clipping mask (masque d'écrêtage)
        pixmap.fill_rect(rect, &paint, Transform::identity(), None);
    }
}