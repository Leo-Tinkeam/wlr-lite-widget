use wlr_lite_widget::{MouseButton, MouseResponse, SizeUnit, SkiaDrawArea, SkiaSurface, StandardDrawArea, SurfaceBox, SurfaceTrait, WidgetBuilder, WidgetPosition, WidgetSize};
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

    let surface = SkiaSurface::new(
        full_size.clone(),
        position_0.clone(),
        render
    ).on_press(|my_struct: &mut MyStruct, button| {
        if button == &MouseButton::LEFT {
            my_struct.clicked = !my_struct.clicked;
            return MouseResponse { do_default: false, need_redraw: true };
        }
        return MouseResponse { do_default: false, need_redraw: false };
    });

    let widget_position = WidgetPosition::Coordinates(SizeUnit::Percent(10f32), SizeUnit::Percent(10f32));

    let mut widget = WidgetBuilder::new(widget_size, widget_position, "MyWidget".to_string())
        .with_skia()
        .build();

    widget.add_surface(surface);

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

fn render(canvas_struct: &mut SkiaDrawArea, widget_width: u32, widget_height: u32, _surface_box: SurfaceBox, app_state: &mut MyStruct) {
    if app_state.clicked {
        canvas_struct.add_rect(0, 0, widget_width, widget_height, 255, 0, 0, 255);
    } else {
        canvas_struct.add_rect(0, 0, widget_width, widget_height, 0, 255, 0, 255);
    }
}