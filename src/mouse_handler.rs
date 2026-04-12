use smithay_client_toolkit::{
    reexports::protocols::wp::cursor_shape::v1::client::wp_cursor_shape_device_v1::Shape, seat::pointer::{PointerEvent, PointerEventKind, PointerHandler}, shell::WaylandSurface
};
use wayland_client::{Connection, QueueHandle, protocol::wl_pointer};
use crate::{Surface, Widget, WidgetState, widget::SharedWidget};

#[derive(PartialEq)]
pub enum MouseButton {
    LEFT,
    RIGHT,
    MIDDLE,
    SIDE, // This is the "next" button
    EXTRA, // This is the "previous" button
}

#[derive(Default)]
pub struct MouseHandler<T> {
    pub(crate) on_press: Option<fn(&mut T, button: &MouseButton) -> MouseResponse>,
    pub(crate) on_release: Option<fn(&mut T, button: &MouseButton) -> MouseResponse>,
}

pub struct MouseResponse {
    pub do_default: bool,
    pub need_redraw: bool,
}

pub fn default_on_press<T: Default>(app_state: &mut T, surfaces: &mut Vec<Surface<T>>, button: &MouseButton, position: (f64, f64)) {
    for surface in surfaces {
        if let Some(surface_box) = &surface.real_size {
            let (x, y) = position;
            if x >= surface_box.min_x as f64 && x <= surface_box.max_x as f64 {
                if y >= surface_box.min_y as f64 && y <= surface_box.max_y as f64 {
                    let mut do_default = true;
                    if let Some(on_press) = surface.mouse_handler.on_press {
                        let mouse_response = on_press(app_state, button);
                        do_default = mouse_response.do_default;
                        if mouse_response.need_redraw {
                            surface.ask_redraw();
                        }
                    }
                    if do_default {
                        default_on_press(app_state, &mut surface.childs_surfaces, button, position);
                    }
                }
            }
        }
    }
}

pub fn default_on_release<T: Default>(app_state: &mut T, surfaces: &mut Vec<Surface<T>>, button: &MouseButton, position: (f64, f64)) { // TODO: This is almost the same as default_on_press, should group them
    for surface in surfaces {
        if let Some(surface_box) = &surface.real_size {
            let (x, y) = position;
            if x >= surface_box.min_x as f64 && x <= surface_box.max_x as f64 {
                if y >= surface_box.min_y as f64 && y <= surface_box.max_y as f64 {
                    let mut do_default = true;
                    if let Some(on_release) = surface.mouse_handler.on_release {
                        let mouse_response = on_release(app_state, button);
                        do_default = mouse_response.do_default;
                        if mouse_response.need_redraw {
                            surface.ask_redraw();
                        }
                    }
                    if do_default {
                        default_on_release(app_state, &mut surface.childs_surfaces, button, position);
                    }
                }
            }
        }
    }
}

impl<T> Widget<T> {
    pub fn on_press(self, func: fn(&mut T, button: &MouseButton) -> MouseResponse) -> Self {
        {
            let mut shared_widget = self.shared_widget.0.lock().unwrap();
            shared_widget.mouse_handler.on_press = Some(func);
        }
        self
    }

    pub fn on_release(self, func: fn(&mut T, button: &MouseButton) -> MouseResponse) -> Self {
        {
            let mut shared_widget = self.shared_widget.0.lock().unwrap();
            shared_widget.mouse_handler.on_release = Some(func);
        }
        self
    }
}

fn parse_mouse_button(code: u32) -> Option<MouseButton> {
    match code { // This come from linux kernel : input-event-code.h
        0x110 => Some(MouseButton::LEFT),
        0x111 => Some(MouseButton::RIGHT),
        0x112 => Some(MouseButton::MIDDLE),
        0x113 => Some(MouseButton::SIDE),
        0x114 => Some(MouseButton::EXTRA),
        _ => None,
    }
}

impl<T: 'static + Default + Send> PointerHandler for WidgetState<T> {
    fn pointer_frame(
        &mut self,
        _conn: &Connection,
        qh: &QueueHandle<Self>,
        pointer: &wl_pointer::WlPointer,
        events: &[PointerEvent],
    ) {
        use PointerEventKind::*;
        for event in events {
            // Ignore events for other surfaces
            match self.layer.clone() {
                None => continue,
                Some(layer) => if &event.surface != layer.wl_surface() {continue},
            }
            match event.kind {
                Enter { serial } => {
                    let device = self.cursor_shape_manager.get_shape_device(pointer, qh);
                    device.set_shape(serial, Shape::Default);
                    device.destroy();
                    // TODO: use this for hover animation
                    // TODO: Appeler un onEnter
                }
                Leave { .. } => {
                    // TODO: use this for hover animation
                    // TODO: Appeler un onLeave
                }
                Motion { .. } => {
                    // TODO: appeler un onMotion
                }
                Press { button, .. } => {
                    let mouse_button = match parse_mouse_button(button) {
                        Some(s) => s,
                        None => return,
                    };
                    let (lock, _) = self.shared_widget.as_ref();
                    let mut shared_widget = lock.lock().unwrap();
                    let mut do_default = true;
                    if let Some(on_press) = shared_widget.mouse_handler.on_press {
                        let mouse_response = on_press(&mut shared_widget.app_state, &mouse_button);
                        do_default = mouse_response.do_default;
                        if mouse_response.need_redraw {
                            shared_widget.ask_redraw(qh);
                        }
                    }
                    if do_default {
                        let SharedWidget {
                            app_state,
                            surfaces,
                            ..
                        } = &mut *shared_widget;
                        default_on_press(app_state, surfaces, &mouse_button, event.position);
                    }
                }
                Release { button, .. } => {
                    let mouse_button = match parse_mouse_button(button) {
                        Some(s) => s,
                        None => return,
                    };
                    let (lock, _) = self.shared_widget.as_ref();
                    let mut shared_widget = lock.lock().unwrap();
                    let mut do_default = true;
                    if let Some(on_release) = shared_widget.mouse_handler.on_release {
                        let mouse_response = on_release(&mut shared_widget.app_state, &mouse_button);
                        do_default = mouse_response.do_default;
                        if mouse_response.need_redraw {
                            shared_widget.ask_redraw(qh);
                        }
                    } 
                    if do_default {
                        let SharedWidget {
                            app_state,
                            surfaces,
                            ..
                        } = &mut *shared_widget;
                        default_on_release(app_state, surfaces, &mouse_button, event.position);
                    }
                }
                Axis { horizontal, vertical, .. } => {
                    // TODO: Appeler un onScroll
                }
            }
        }
    }
}