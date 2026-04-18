use std::{marker::PhantomData, sync::{atomic::{AtomicI32, Ordering}, mpsc::Sender}};
use smithay_client_toolkit::{seat::pointer::AxisScroll, shell::{WaylandSurface, wlr_layer::LayerSurface}};

use crate::{MouseButton, MouseHandler, MouseResponse, WidgetPosition, WidgetSize, widget::WidgetEvent, widget_builder::DrawAreaType};

static NEXT_SURFACE_ID: AtomicI32 = AtomicI32::new(1);

pub fn no_render<'a, T, U: DrawAreaType>(_canvas: &mut U::Type<'a>, _widget_width: u32, _widget_height: u32, _surface_box: SurfaceBox, _app_state: &mut T) {}

pub struct SurfaceData<T, U: DrawAreaType, V> {
    pub(crate) id: i32,
    pub(crate) size: WidgetSize,
    pub(crate) position: WidgetPosition,
    pub(crate) need_redraw: bool,
    event_sender: Option<Sender<WidgetEvent>>,
    pub(crate) mouse_handler: MouseHandler<T>,
    pub(crate) real_size: Option<SurfaceBox>,
    pub(crate) childs_surfaces: Vec<V>,
    _marker: PhantomData<U>,
}

pub trait SurfaceTrait<T, U: DrawAreaType>: Sized {
    fn get_surface_data_mut(&mut self) -> &mut SurfaceData<T, U, Self>;

    fn get_surface_data(&self) -> &SurfaceData<T, U, Self>;

    fn render<'a>(&mut self, draw_area: &mut U::Type<'a>, width: u32, height: u32, surface_box: SurfaceBox, app_state: &mut T);

    fn on_enter(mut self, func: fn(&mut T) -> MouseResponse) -> Self {
        let surface_data = self.get_surface_data_mut();
        surface_data.mouse_handler.on_enter = Some(func);
        self
    }

    fn on_leave(mut self, func: fn(&mut T) -> MouseResponse) -> Self {
        let surface_data = self.get_surface_data_mut();
        surface_data.mouse_handler.on_leave = Some(func);
        self
    }

    fn on_motion(mut self, func: fn(&mut T, (f64, f64)) -> MouseResponse) -> Self {
        let surface_data = self.get_surface_data_mut();
        surface_data.mouse_handler.on_motion = Some(func);
        self
    }

    fn on_press(mut self, func: fn(&mut T, button: &MouseButton) -> MouseResponse) -> Self {
        let surface_data = self.get_surface_data_mut();
        surface_data.mouse_handler.on_press = Some(func);
        self
    }

    fn on_release(mut self, func: fn(&mut T, button: &MouseButton) -> MouseResponse) -> Self {
        let surface_data = self.get_surface_data_mut();
        surface_data.mouse_handler.on_release = Some(func);
        self
    }

    fn on_scroll(mut self, func: fn(&mut T, AxisScroll, AxisScroll) -> MouseResponse) -> Self {
        let surface_data = self.get_surface_data_mut();
        surface_data.mouse_handler.on_scroll = Some(func);
        self
    }

    fn edit_size(&mut self, new_size: WidgetSize) {
        let surface_data = self.get_surface_data_mut();
        surface_data.size = new_size;

        self.ask_redraw();
    }

    fn edit_position(&mut self, new_position: WidgetPosition) {
        let surface_data = self.get_surface_data_mut();
        surface_data.position = new_position;

        self.ask_redraw();
    }

    fn to_front_of(&mut self, other_surface: &mut Self) {
        let surface_data = self.get_surface_data_mut();
        let orher_surface_data = other_surface.get_surface_data_mut();
        if surface_data.id < orher_surface_data.id {
            let temp_id = surface_data.id;
            surface_data.id = orher_surface_data.id;
            orher_surface_data.id = temp_id;

            self.ask_redraw();
            other_surface.ask_redraw();
        }
    }

    fn ask_redraw(&mut self) {
        let surface_data = self.get_surface_data_mut();
        surface_data.need_redraw = true;
        if let Some(sender) = surface_data.event_sender.as_mut() {
            if sender.send(WidgetEvent::Redraw).is_err() {
                println!("Error: redraw not sent");
            }
        }
    }

    fn update_size(&mut self, parent_width: u32, parent_height: u32, parent_x: u32, parent_y: u32) {
        let surface_data = self.get_surface_data_mut();
        let (size_x, size_y) = surface_data.size.get_dimension(parent_width, parent_height);
        let (min_x, min_y) = surface_data.position.get_coordinates(parent_width, parent_height, (size_x, size_y));
        let (min_x, min_y) = (min_x as u32, min_y as u32);
        surface_data.real_size = Some(SurfaceBox {
            min_x: parent_x+min_x,
            max_x: parent_x+min_x+size_x,
            min_y: parent_y+min_y,
            max_y: parent_y+min_y+size_y,
        });
        for surface in &mut surface_data.childs_surfaces {
            surface.update_size(size_x, size_y, min_x, min_y);
        }
        // TODO: ask for redraw ? (or maybe not here)
    }

    fn set_event_sender(&mut self, event_sender: Sender<WidgetEvent>) {
        let surface_data = self.get_surface_data_mut();
        for surface in &mut surface_data.childs_surfaces {
            surface.set_event_sender(event_sender.clone());
        }
        surface_data.event_sender = Some(event_sender);
    }

    fn add_surface(&mut self, mut surface: Self) {
        let surface_data = self.get_surface_data_mut();
        if let Some(surface_box) = surface_data.real_size {
            let (xs, ys, xe, ye) = (surface_box.min_x, surface_box.min_y, surface_box.max_x, surface_box.max_y);
            surface.update_size(xe-xs, ye-ys, xs, ys);
        }
        if let Some(event_sender) = surface_data.event_sender.clone() {
            surface.set_event_sender(event_sender);
        }
        surface_data.childs_surfaces.push(surface);
    }

    fn draw(&mut self, app_state: &mut T, parent_width: u32, parent_height: u32, layer: &LayerSurface, draw_area: &mut U::Type<'_>, total_width: u32, total_height: u32, force_redraw: bool,
        draw_surfaces: fn(&mut Vec<Self>, &mut T, u32, u32, &LayerSurface, &mut U::Type<'_>, u32, u32, bool)) {
        let (surface_width, surface_height) = self.get_surface_data_mut().size.get_dimension(parent_width, parent_height);
        let mut force_child_redraw = false;
        if self.get_surface_data_mut().need_redraw || force_redraw {
            force_child_redraw = true;
            if let Some(real_size) = self.get_surface_data_mut().real_size {
                self.get_surface_data_mut().need_redraw = false;
                self.render(draw_area, total_width, total_height, real_size, app_state);
                layer.wl_surface().damage_buffer(real_size.min_x as i32, real_size.min_y as i32, surface_width as i32, surface_height as i32); // TODO: maybe surface.render should return the area to damage (to accept damaging more than his area for shadow or restrict to a smaller area)
            }
        }
        draw_surfaces(&mut self.get_surface_data_mut().childs_surfaces, app_state, surface_width, surface_height, layer, draw_area, total_width, total_height, force_child_redraw);
    }
}

pub struct Surface<T, U: DrawAreaType> {
    pub(crate) render: Box<dyn for<'a> FnMut(&mut U::Type<'a>, u32, u32, SurfaceBox, &mut T) + Send>, // TODO: Help user to create these for exemple fill_color() and a custom type for advanced shapes
    surface_data: SurfaceData<T, U, Self>
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

impl<T, U: DrawAreaType> SurfaceTrait<T, U> for Surface<T, U> {
    fn get_surface_data_mut(&mut self) -> &mut SurfaceData<T, U, Surface<T, U>> {
        &mut self.surface_data
    }

    fn get_surface_data(&self) -> &SurfaceData<T, U, Surface<T, U>> {
        &self.surface_data
    }

    fn render<'a>(&mut self, draw_area: &mut <U as DrawAreaType>::Type<'a>, width: u32, height: u32, surface_box: SurfaceBox, app_state: &mut T) {
        (self.render)(draw_area, width, height, surface_box, app_state)
    }
}

impl<T: Default, U: DrawAreaType> Surface<T, U> {
    pub fn new<F>(size: WidgetSize, position: WidgetPosition, render: F) -> Self
    where
        F: for<'a> FnMut(&mut U::Type<'a>, u32, u32, SurfaceBox, &mut T) + 'static + Send
    {
        let id = NEXT_SURFACE_ID.fetch_add(1, Ordering::Relaxed);

        Surface {
            render: Box::new(render),
            surface_data: SurfaceData {
                id,
                size,
                position,
                need_redraw: true,
                event_sender: None,
                mouse_handler: MouseHandler::default(),
                real_size: None,
                childs_surfaces: vec![],
                _marker: PhantomData,
            },
        }
    }

    // We should be able to animate this on "hover" (maybe render with a 0-1 float)
}

impl<T: Default, U: DrawAreaType> Surface<T, U>  {
    pub fn edit_render<F>(&mut self, new_render: F)
    where
        F: for<'a> FnMut(&mut U::Type<'a>, u32, u32, SurfaceBox, &mut T) + 'static + Send
    {
        self.render = Box::new(new_render);

        self.ask_redraw();
    }
}