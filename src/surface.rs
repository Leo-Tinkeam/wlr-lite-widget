use std::sync::{atomic::{AtomicI32, Ordering}, mpsc::Sender};
use crate::{MouseButton, MouseHandler, MouseResponse, WidgetPosition, WidgetSize, widget::WidgetEvent};

static NEXT_SURFACE_ID: AtomicI32 = AtomicI32::new(1);

pub struct Surface<T> { // TODO: optionnal render for virtual surfaces (that only contains others) or maybe use a constant NoRender
    pub(crate) id: i32,
    pub(crate) size: WidgetSize,
    pub(crate) position: WidgetPosition,
    pub(crate) render: fn(&mut [u8], u32, u32, SurfaceBox, &mut T), // TODO: Help user to create these for exemple fill_color() and a custom type for advanced shapes
    pub(crate) need_redraw: bool,
    event_sender: Option<Sender<WidgetEvent>>,
    pub(crate) mouse_handler: MouseHandler<T>,
    pub(crate) real_size: Option<SurfaceBox>,
    pub(crate) childs_surfaces: Vec<Surface<T>>,
}

#[derive(Clone, Copy)]
pub struct SurfaceBox {
    pub(crate) min_x: u32,
    pub(crate) max_x: u32,
    pub(crate) min_y: u32,
    pub(crate) max_y: u32,
}

impl SurfaceBox {
    pub fn get_xywh(&self) -> (f32, f32, f32, f32) {
        let (min_x, max_x, min_y, max_y) = (self.min_x as f32, self.max_x as f32, self.min_y as f32, self.max_y as f32);
        return (min_x, min_y, max_x-min_x, max_y-min_y);
    }
}

impl<T: Default> Surface<T> {
    pub fn new(size: WidgetSize, position: WidgetPosition, render: fn(&mut [u8], u32, u32, SurfaceBox, &mut T)) -> Self {
        let id = NEXT_SURFACE_ID.fetch_add(1, Ordering::Relaxed);

        Surface {
            id,
            size,
            position,
            render,
            need_redraw: true,
            event_sender: None,
            mouse_handler: MouseHandler::default(),
            real_size: None,
            childs_surfaces: vec![],
        }
    }

    pub fn on_press(mut self, func: fn(&mut T, button: &MouseButton) -> MouseResponse) -> Self {
        self.mouse_handler.on_press = Some(func);
        self
    }

    pub fn on_release(mut self, func: fn(&mut T, button: &MouseButton) -> MouseResponse) -> Self {
        self.mouse_handler.on_release = Some(func);
        self
    }

    pub fn edit_size(&mut self, new_size: WidgetSize) {
        self.size = new_size;

        self.ask_redraw();
    }

    pub fn edit_position(&mut self, new_position: WidgetPosition) {
        self.position = new_position;

        self.ask_redraw();
    }

    pub fn edit_render(&mut self, new_render: fn(&mut [u8], u32, u32, SurfaceBox, &mut T)) {
        self.render = new_render;

        self.ask_redraw();
    }

    pub fn to_front_of(&mut self, other_surface: &mut Surface<T>) {
        if self.id < other_surface.id {
            let temp_id = self.id;
            self.id = other_surface.id;
            other_surface.id = temp_id;

            self.ask_redraw();
            other_surface.ask_redraw();
        }
    }

    pub(crate) fn ask_redraw(&mut self) {
        self.need_redraw = true;
        if let Some(sender) = self.event_sender.as_mut() {
            if sender.send(WidgetEvent::Redraw).is_err() {
                println!("Error: redraw not sent");
            }
        }
    }

    pub(crate) fn update_size(&mut self, parent_width: u32, parent_height: u32, parent_x: u32, parent_y: u32) {
        let (size_x, size_y) = self.size.get_dimension(parent_width, parent_height);
        let (min_x, min_y) = self.position.get_coordinates(parent_width, parent_height, (size_x, size_y));
        let (min_x, min_y) = (min_x as u32, min_y as u32);
        self.real_size = Some(SurfaceBox {
            min_x: parent_x+min_x,
            max_x: parent_x+min_x+size_x,
            min_y: parent_y+min_y,
            max_y: parent_y+min_y+size_y,
        });
        for surface in &mut self.childs_surfaces {
            surface.update_size(size_x, size_y, min_x, min_y);
        }
        // TODO: ask for redraw ? (or maybe not here)
    }

    pub(crate) fn set_event_sender(&mut self, event_sender: Sender<WidgetEvent>) {
        for surface in &mut self.childs_surfaces {
            surface.set_event_sender(event_sender.clone());
        }
        self.event_sender = Some(event_sender);
    }

    pub fn add_surface(&mut self, mut surface: Surface<T>) {
        if let Some(surface_box) = self.real_size {
            let (xs, ys, xe, ye) = (surface_box.min_x, surface_box.min_y, surface_box.max_x, surface_box.max_y);
            surface.update_size(xe-xs, ye-ys, xs, ys);
        }
        if let Some(event_sender) = self.event_sender.clone() {
            surface.set_event_sender(event_sender);
        }
        self.childs_surfaces.push(surface);
    }

    // We should be able to animate this on "hover" (maybe render with a 0-1 float)
}