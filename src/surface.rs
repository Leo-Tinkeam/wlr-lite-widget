use std::marker::PhantomData;
use crate::{MouseHandler, SurfaceBox, SurfaceData, SurfaceTrait, WidgetPosition, WidgetSize, get_next_surface_id, widget_builder::DrawAreaType};

pub fn no_render<'a, T, U: DrawAreaType>(_canvas: &mut U::Type<'a>, _widget_width: u32, _widget_height: u32, _surface_box: SurfaceBox, _app_state: &mut T) {}

pub struct WithCanvasRender;

pub struct CanvasType<'a> {
    pub canvas: &'a mut [u8],
}

impl DrawAreaType for WithCanvasRender {
    type Type<'a> = CanvasType<'a>;

    fn get_draw_area<'a>(canvas: &'a mut [u8], _width: u32, _height: u32) -> Self::Type<'a> {
        CanvasType {
            canvas
        }
    }
}

pub struct Surface<T, U: DrawAreaType> {
    pub(crate) render: Box<dyn for<'a> FnMut(&mut U::Type<'a>, u32, u32, SurfaceBox, &mut T) + Send>,
    surface_data: SurfaceData<T, U, Self>
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
        let id = get_next_surface_id();

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
