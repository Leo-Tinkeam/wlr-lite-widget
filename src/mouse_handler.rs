use smithay_client_toolkit::{
    reexports::protocols::wp::cursor_shape::v1::client::wp_cursor_shape_device_v1::Shape, seat::pointer::{PointerEvent, PointerEventKind, PointerHandler}, shell::WaylandSurface
};
use wayland_client::{Connection, QueueHandle, protocol::wl_pointer};
use crate::{Surface, Widget, WidgetState, widget::SharedWidget};

#[derive(PartialEq, Clone)]
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

fn call_mouse_action<T, F, D>(on_action_option: Option<F>, on_action_default: D, app_state: &mut T, surfaces: &mut Vec<Surface<T>>) -> bool
where 
    F: FnOnce(&mut T) -> MouseResponse,
    D: FnOnce(&mut T, &mut Vec<Surface<T>>),
{
    let mut need_redraw = false;
    let mut do_default = true;
    if let Some(on_action) = on_action_option {
        let mouse_response = on_action(app_state);
        do_default = mouse_response.do_default;
        if mouse_response.need_redraw {
            need_redraw = true;
        }
    }
    if do_default {
        on_action_default(app_state, surfaces);
    }
    need_redraw
}

pub fn default_on_press<T: Default>(app_state: &mut T, surfaces: &mut Vec<Surface<T>>, button: &MouseButton, position: (f64, f64)) {
    for surface in surfaces {
        if let Some(surface_box) = &surface.real_size {
            let (x, y) = position;
            if x >= surface_box.min_x as f64 && x <= surface_box.max_x as f64 {
                if y >= surface_box.min_y as f64 && y <= surface_box.max_y as f64 {
                    if call_mouse_action(
                        surface.mouse_handler.on_press.map(|on_press| move |app_state: &mut T| { on_press(app_state, button) }),
                        |app_state, surfaces| default_on_press(app_state, surfaces, button, position),
                        app_state,
                        &mut surface.childs_surfaces
                    ) {
                        surface.ask_redraw();
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
                    if call_mouse_action(
                        surface.mouse_handler.on_release.map(|on_release| move |app_state: &mut T| { on_release(app_state, button) }),
                        |app_state, surfaces| default_on_release(app_state, surfaces, button, position),
                        app_state,
                        &mut surface.childs_surfaces
                    ) {
                        surface.ask_redraw();
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
                    // Set mouse texture
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

                    let SharedWidget {
                        app_state,
                        surfaces,
                        mouse_handler,
                        ..
                    } = &mut *shared_widget;
                    let mouse_button_clone = mouse_button.clone();
                    if call_mouse_action(
                        mouse_handler.on_press.map(|on_press| move |app_state: &mut T| { on_press(app_state, &mouse_button) }),
                        |app_state, surfaces| default_on_press(app_state, surfaces, &mouse_button_clone, event.position),
                        app_state,
                        surfaces
                    ) {
                        shared_widget.ask_redraw(qh)
                    }
                }
                Release { button, .. } => {
                    let mouse_button = match parse_mouse_button(button) {
                        Some(s) => s,
                        None => return,
                    };
                    let (lock, _) = self.shared_widget.as_ref();
                    let mut shared_widget = lock.lock().unwrap();

                    let SharedWidget {
                        app_state,
                        surfaces,
                        mouse_handler,
                        ..
                    } = &mut *shared_widget;
                    let mouse_button_clone = mouse_button.clone();
                    if call_mouse_action(
                        mouse_handler.on_release.map(|on_realease| move |app_state: &mut T| { on_realease(app_state, &mouse_button) }),
                        |app_state, surfaces| default_on_release(app_state, surfaces, &mouse_button_clone, event.position),
                        app_state,
                        surfaces
                    ) {
                        shared_widget.ask_redraw(qh)
                    }
                }
                Axis { horizontal, vertical, .. } => {
                    // TODO: Appeler un onScroll
                }
            }
        }
    }
}