use wlr_lite_widget::{DrawAreaType, Layer, SizeUnit, Surface, SurfaceBox, WidgetAnchor, WidgetBuilder, WidgetMargin, WidgetPosition, WidgetSize};
use tiny_skia::{Color, FillRule, LineCap, Paint, PathBuilder, PixmapMut, Rect, Stroke, Transform};
use std::thread;
use std::time::Duration;
use chrono::{DateTime, Local, Timelike};
use std::f32::consts::PI;

#[derive(Default, Clone)]
pub struct MyStruct {
    time: DateTime<Local>,
}

pub struct WithMyFunction;

impl DrawAreaType for WithMyFunction {
    type Type<'a> = PixmapMut<'a>;

    fn get_draw_area<'a>(canvas: &'a mut [u8], width: u32, height: u32) -> Self::Type<'a> {
        PixmapMut::from_bytes(
            canvas, 
            width,
            height,
        ).expect("Erreur taille buffer / stride")
    }
}

fn main() {
    println!("--- Début de l'exemple ---");

    let full_size = WidgetSize {
        width: SizeUnit::Percent(100f32),
        height: SizeUnit::Percent(100f32),
    };
    let position_0 = WidgetPosition::Coordinates(SizeUnit::Pixel(0), SizeUnit::Pixel(0));
    let surface = Surface::new(
        full_size,
        position_0,
        render
    );

    let widget_size = WidgetSize {
        width: SizeUnit::Pixel(200),
        height: SizeUnit::Pixel(200),
    };
    let position = WidgetPosition::Anchor(
        WidgetAnchor::TopLeft,
        Some(WidgetMargin {
            top: Some(SizeUnit::Pixel(50)),
            left: Some(SizeUnit::Pixel(50)),
            bottom: None,
            right: None,
        }));
    let mut widget = WidgetBuilder::new::<WithMyFunction>(widget_size, position, "MyWidget".to_string())
        .at_layer(Layer::Overlay)
        .build();
    widget.add_surface(surface);

    let thread_widget = widget.clone();
    thread::spawn(move || {
        let mut my_state = MyStruct { time: Local::now() };
        loop {
            my_state.time = Local::now();
            thread_widget.update_app_state(my_state.clone());
            thread_widget.force_redraw();
            thread::sleep(Duration::from_secs(1));
            if !thread_widget.is_running() {
                break;
            }
        }
    });

    widget.run();

    println!("--- Fin de l'exemple ---");
}

fn render(pixmap: &mut PixmapMut, widget_width: u32, widget_height: u32, _surface_box: SurfaceBox, app_state: &mut MyStruct) {
    let cx = widget_width as f32 / 2.0;
    let cy = widget_height as f32 / 2.0;
    let radius = cx.min(cy) * 0.90;

    let bg_paint = make_paint(Color::from_rgba8(0, 0, 0, 0));
    let face_paint = make_paint(Color::from_rgba8(255, 255, 255, 255));
    let details_paint = make_paint(Color::from_rgba8(30, 30, 30, 255));
    let second_paint = make_paint(Color::from_rgba8(30, 30, 200, 255)); // Default wayland is BGRA so this is red

    let bg_rect = Rect::from_xywh(0.0, 0.0, widget_width as f32, widget_height as f32).unwrap();
    pixmap.fill_rect(bg_rect, &bg_paint, Transform::identity(), None);
    draw_circle(pixmap, cx, cy, radius, &face_paint);

    // Graduations
    for i in 0..60 {
        let angle = (i as f32) * (2.0 * PI / 60.0) - PI / 2.0;
        let is_hour_tick = i % 5 == 0;
        let inner_r = if is_hour_tick { radius * 0.82 } else { radius * 0.88 };
        let outer_r = radius * 0.93;
        let width = if is_hour_tick { 3.0 } else { 1.0 };

        let x1 = cx + inner_r * angle.cos();
        let y1 = cy + inner_r * angle.sin();
        let x2 = cx + outer_r * angle.cos();
        let y2 = cy + outer_r * angle.sin();

        draw_line(pixmap, x1, y1, x2, y2, width, &details_paint);
    }

    let hour = app_state.time.hour() % 12;
    let minute = app_state.time.minute();
    let second = app_state.time.second();
    let hour_angle = ((hour as f32 + minute as f32 / 60.0) * (2.0 * PI / 12.0)) - PI / 2.0;
    let minute_angle = ((minute as f32 + second as f32 / 60.0) * (2.0 * PI / 60.0)) - PI / 2.0;
    let second_angle = ((second as f32) * (2.0 * PI / 60.0)) - PI / 2.0;

    // Hours and minutes
    draw_hand(pixmap, cx, cy, hour_angle, radius * 0.50, 5.5, &details_paint);
    draw_hand(pixmap, cx, cy, minute_angle, radius * 0.72, 3.5, &details_paint);

    // Seconds
    let tail_angle = second_angle + PI;
    draw_hand(pixmap, cx, cy, tail_angle, radius * 0.15, 1.5, &second_paint);
    draw_hand(pixmap, cx, cy, second_angle, radius * 0.80, 1.5, &second_paint);

    // Central circles
    draw_circle(pixmap, cx, cy, 5.0, &details_paint);
    draw_circle(pixmap, cx, cy, 2.5, &second_paint);
}

fn make_paint(color: Color) -> Paint<'static> {
    let mut paint = Paint::default();
    paint.set_color(color);
    paint.anti_alias = true;
    paint
}

fn draw_line(
    pixmap: &mut PixmapMut,
    x1: f32, y1: f32,
    x2: f32, y2: f32,
    width: f32,
    paint: &Paint,
) {
    let mut pb = PathBuilder::new();
    pb.move_to(x1, y1);
    pb.line_to(x2, y2);
    let path = pb.finish().unwrap();
    let mut stroke = Stroke::default();
    stroke.width = width;
    stroke.line_cap = LineCap::Round;
    pixmap.stroke_path(&path, paint, &stroke, Transform::identity(), None);
}

fn draw_hand(
    pixmap: &mut PixmapMut,
    cx: f32, cy: f32,
    angle: f32,
    length: f32,
    width: f32,
    paint: &Paint,
) {
    let x = cx + length * angle.cos();
    let y = cy + length * angle.sin();
    draw_line(pixmap, cx, cy, x, y, width, paint);
}

fn draw_circle(
    pixmap: &mut PixmapMut,
    cx: f32, cy: f32,
    r: f32,
    paint: &Paint,
) {
    let mut pb = PathBuilder::new();
    pb.push_circle(cx, cy, r);
    let path = pb.finish().unwrap();
    pixmap.fill_path(&path, paint, FillRule::Winding, Transform::identity(), None);
}