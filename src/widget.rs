use smithay_client_toolkit::{
    compositor::{CompositorHandler, CompositorState},
    delegate_compositor, delegate_layer, delegate_output,
    delegate_pointer, delegate_registry, delegate_seat,
    delegate_shm,
    output::{OutputHandler, OutputState},
    registry::{ProvidesRegistryState, RegistryState},
    registry_handlers,
    seat::{Capability, SeatHandler, SeatState, pointer::cursor_shape::CursorShapeManager,},
    shell::{
        WaylandSurface,
        wlr_layer::{
            Anchor, Layer, LayerShell,
            LayerShellHandler, LayerSurface,LayerSurfaceConfigure,
        },
    },
    shm::{Shm, ShmHandler, slot::SlotPool},
};
use std::sync::{Arc, Condvar, Mutex, mpsc::Sender};
use wayland_client::{
    Connection, Dispatch, QueueHandle, protocol::{wl_callback, wl_output, wl_pointer, wl_seat, wl_shm, wl_surface}
};
use crate::{Margin, MouseHandler, Surface, WidgetSize};

#[derive(Clone)]
pub struct Widget<T> {
    pub(crate) shared_widget: Arc<(Mutex<SharedWidget<T>>, Condvar)>,
}

impl<T: 'static + Default + Send> Widget<T> {
    pub fn add_surface(&mut self, mut surface: Surface<T>) {
        let mut shared_widget = self.shared_widget.0.lock().unwrap();
        if let (Some(width), Some(height)) = (shared_widget.width, shared_widget.height) {
            surface.update_size(width, height, 0, 0);
        }
        surface.set_event_sender(shared_widget.event_sender.clone());
        shared_widget.surfaces.push(surface);
    }

    pub fn run(&self) {
        // TODO: this is allowing only one widget to be displayed, everithing should be good to allow several widget
        // TODO: so we need a container with a Vec<Widget> that run them all
        let (lock, cvar) = self.shared_widget.as_ref();
        let mut shared_widget = lock.lock().unwrap();
        while !shared_widget.exit {
            // cvar.wait() is unlocking the .lock() above while waiting
            shared_widget = cvar.wait(shared_widget).unwrap();
        }
    }

    pub fn is_running(&self) -> bool {
        let shared_widget = self.shared_widget.0.lock().unwrap();
        if shared_widget.exit {
            return false;
        }
        true
    }

    pub fn force_redraw(&self) {
        let mut shared_widget = self.shared_widget.0.lock().unwrap();
        shared_widget.force_redraw = true;
        if shared_widget.event_sender.send(WidgetEvent::Redraw).is_err() {
            println!("Error: redraw not sent");
        };
    }

    pub fn update_app_state(&self, new_state: T) {
        let mut shared_widget = self.shared_widget.0.lock().unwrap();
        shared_widget.app_state = new_state;
    }
}

pub(crate) struct SharedWidget<T> {
    pub(crate) exit: bool, 
    pub(crate) app_state: T,
    pub(crate) surfaces: Vec<Surface<T>>,
    pub(crate) event_sender: Sender<WidgetEvent>,
    pub(crate) frame_asked: bool,
    pub(crate) force_redraw: bool,
    pub(crate) wl_surface: Option<wl_surface::WlSurface>,
    pub(crate) conn: Connection,

    pub(crate) mouse_handler: MouseHandler<T>,
    pub(crate) width: Option<u32>,
    pub(crate) height: Option<u32>,
}

impl<T: 'static + Default + Send> SharedWidget<T> {
    pub(crate) fn ask_redraw(&mut self, qh: &QueueHandle<WidgetState<T>>) {
        if !self.frame_asked {
            if let Some(surface) = self.wl_surface.as_mut() {
                surface.frame(qh, FrameRequest::Redraw);
                surface.commit();
                self.conn.flush().expect("Disconnected from wayland");
                self.frame_asked = true;
            }
        }
    }
}

pub(crate) enum WidgetEvent {
    Redraw,
    Exit,
}

pub(crate) struct WidgetState<T> {
    pub(crate) registry_state: RegistryState,
    pub(crate) seat_state: SeatState,
    pub(crate) output_state: OutputState,
    pub(crate) shm: Shm,
    pub(crate) compositor: CompositorState,
    pub(crate) layer_shell: LayerShell,

    pub(crate) need_redraw: bool,
    pub(crate) pool: Option<SlotPool>,
    pub(crate) layer: Option<LayerSurface>,
    pub(crate) cursor_shape_manager: CursorShapeManager,
    pub(crate) pointer: Option<wl_pointer::WlPointer>,

    pub(crate) widget_size: WidgetSize,
    pub(crate) widget_name: String,
    pub(crate) widget_layer: Layer,
    pub(crate) widget_anchor: Option<Anchor>,
    pub(crate) margin: Margin,

    pub(crate) shared_widget: Arc<(Mutex<SharedWidget<T>>, Condvar)>,
}

impl<T: 'static + Default + Send> WidgetState<T> {
    pub fn draw(&mut self) {
        let layer = match self.layer.clone() {
            None => return,
            Some(layer) => layer,
        };
        
        {
            // Protected access to shared_widget for the duration of this block
            let mut shared_widget = self.shared_widget.0.lock().unwrap();

            let width = shared_widget.width.unwrap();
            let height = shared_widget.height.unwrap();
            let stride = width as i32 * 4;
            let (buffer, canvas) = self.pool
                .as_mut()
                .unwrap()
                .create_buffer(width as i32, height as i32, stride, wl_shm::Format::Argb8888)
                .expect("Error while creating buffer");

            // Render with the user render function
            let SharedWidget {
                app_state,
                surfaces,
                force_redraw,
                ..
            } = &mut *shared_widget;
            draw_surfaces(surfaces, app_state, width, height, &layer, canvas, width, height, *force_redraw);
            shared_widget.force_redraw = false;

            // Attach and commit to present.
            buffer.attach_to(layer.wl_surface()).expect("buffer attach");
        }
        
        layer.commit();
    }

    fn update_size(&mut self, new_width: u32, new_height: u32) {
        let mut shared_widget = self.shared_widget.0.lock().unwrap();
        (shared_widget.width, shared_widget.height) = (Some(new_width), Some(new_height));
        for surface in &mut shared_widget.surfaces {
            surface.update_size(new_width, new_height, 0, 0);
        }
    }
}

fn draw_surfaces<T>(surfaces: &mut Vec<Surface<T>>, app_state: &mut T, parent_width: u32, parent_height: u32, layer: &LayerSurface, canvas: &mut [u8], total_width: u32, total_height: u32, force_redraw: bool) {
    surfaces.sort_by_key(|s| s.id); // TODO: only call this when to_front_of is call on a Surface ?
    for surface in surfaces {
        // TODO: should redraw surfaces without need_redraw if they are above a redrawed surfaces (since they are sorted before the for loop, this should be easy)
        let (surface_width, surface_height) = surface.size.get_dimension(parent_width, parent_height);
        let mut force_child_redraw = false;
        if surface.need_redraw || force_redraw {
            force_child_redraw = true;
            if let Some(real_size) = surface.real_size {
                surface.need_redraw = false;
                (surface.render)(canvas, total_width, total_height, real_size, app_state);
                layer.wl_surface().damage_buffer(real_size.min_x as i32, real_size.min_y as i32, surface_width as i32, surface_height as i32); // TODO: maybe surface.render should return the area to damage (to accept damaging more than his area for shadow or restrict to a smaller area)
            }
        }

        draw_surfaces(&mut surface.childs_surfaces, app_state, surface_width, surface_height, layer, canvas, total_width, total_height, force_child_redraw);
    }
}

impl<T: 'static + Default + Send> CompositorHandler for WidgetState<T> {
    fn scale_factor_changed(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _surface: &wl_surface::WlSurface,
        _new_factor: i32,
    ) {
        // This is not needed since scale change also call update_output from OutputHandler that is implemented
    }

    fn transform_changed(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _surface: &wl_surface::WlSurface,
        _new_transform: wl_output::Transform,
    ) {
        // This is not needed since rotation also call update_output from OutputHandler that is implemented
    }

    fn frame(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _surface: &wl_surface::WlSurface,
        _time: u32,
    ) {
        self.draw();
    }

    fn surface_enter(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _surface: &wl_surface::WlSurface,
        _output: &wl_output::WlOutput,
    ) {}

    fn surface_leave(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _surface: &wl_surface::WlSurface,
        _output: &wl_output::WlOutput,
    ) {}
}

impl<T: 'static + Default + Send> OutputHandler for WidgetState<T> {
    fn output_state(&mut self) -> &mut OutputState {
        &mut self.output_state
    }

    fn new_output(
        &mut self,
        _conn: &Connection,
        qh: &QueueHandle<Self>,
        output: wl_output::WlOutput,
    ) {
        // TODO: Must be a problem with several screen (can't test now) -> main screen should be at location (0, 0) ?
        // TODO: If we achieve to choose a screen, we may use the wlr-output-management extension that gives zwlr_output_head_v1
        if self.layer.is_none() {
            if let Some(info) = self.output_state.info(&output) {
                let surface = self.compositor.create_surface(&qh);
                let new_layer =
                    self.layer_shell.create_layer_surface(&qh, surface.clone(), self.widget_layer, Some(self.widget_name.clone()), Some(&output));
                
                let (screen_width, screen_height) = (info.logical_size.unwrap().0 as u32, info.logical_size.unwrap().1 as u32);
                if let Some(anchor) = self.widget_anchor {
                    new_layer.set_anchor(anchor);
                    let (top, right, bottom, left) = self.margin.get_margin(screen_width, screen_height);
                    new_layer.set_margin(top, right, bottom, left);
                }
                let (width, height) = self.widget_size.get_dimension(screen_width, screen_height);
                new_layer.set_size(width, height);
                self.update_size(width, height);

                // In order for the layer surface to be mapped, we need to perform an initial commit with no attached\
                // buffer. For more info, see WaylandSurface::commit
                // The compositor will respond with an initial configure that we can then use to present to the layer
                // surface with the correct options.
                new_layer.commit();

                self.pool = Some(SlotPool::new((width * height * 4).try_into().expect("Too large dimension"), &self.shm).expect("Failed to create pool"));
                self.layer = Some(new_layer);
                {
                    let (lock, _) = self.shared_widget.as_ref();
                    let mut shared_widget = lock.lock().unwrap();
                    shared_widget.wl_surface = Some(surface);
                }
                self.need_redraw = true;
            }
        }
    }

    fn update_output(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        output: wl_output::WlOutput,
    ) {
        // When the screen is rotated (transform), this goes from 3840x2160 to 2160x3840 (no need to consider it)
        // This is the size after division by scale_factor (no need to consider it)
        let (screen_width, screen_height) = if let Some(info) = self.output_state.info(&output) {
            (info.logical_size.unwrap().0 as u32, info.logical_size.unwrap().1 as u32)
        } else {
            return;
        };
        
        if self.widget_anchor.is_some() {
            let (top, right, bottom, left) = self.margin.get_margin(screen_width, screen_height);
            self.layer.as_mut().unwrap().set_margin(top, right, bottom, left);
        }
        let (width, height) = self.widget_size.get_dimension(screen_width, screen_height);
        self.layer.as_mut().unwrap().set_size(width, height);
        self.update_size(width, height);

        self.layer.as_mut().unwrap().commit();
        self.pool = Some(SlotPool::new((width * height * 4).try_into().expect("Too large dimension"), &self.shm).expect("Failed to create pool"));
        self.need_redraw = true;
    }

    fn output_destroyed(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _output: wl_output::WlOutput,
    ) {
        // TODO: Must be a problem with several screen (can't test now) -> main screen should be at location (0, 0) ?
        // TODO: If we achieve to choose a screen, we may use the wlr-output-management extension that gives zwlr_output_head_v1
        self.layer = None;
    }
}

impl<T: 'static + Default + Send> LayerShellHandler for WidgetState<T> {
    fn closed(&mut self, _conn: &Connection, _qh: &QueueHandle<Self>, _layer: &LayerSurface) {
        let mut shared_widget = self.shared_widget.0.lock().unwrap();
        if shared_widget.event_sender.send(WidgetEvent::Exit).is_err() {
            println!("Error: exit not sent");
        }
        shared_widget.exit = true;
    }

    fn configure(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _layer: &LayerSurface,
        _configure: LayerSurfaceConfigure,
        _serial: u32,
    ) {
        if self.need_redraw {
            self.need_redraw = false;
            self.draw();
        }
    }
}

impl<T: 'static + Default + Send> SeatHandler for WidgetState<T> {
    fn seat_state(&mut self) -> &mut SeatState {
        &mut self.seat_state
    }

    fn new_seat(&mut self, _: &Connection, _: &QueueHandle<Self>, _: wl_seat::WlSeat) {
        // Not needed for our widget
    }

    fn new_capability(
        &mut self,
        _conn: &Connection,
        qh: &QueueHandle<Self>,
        seat: wl_seat::WlSeat,
        capability: Capability,
    ) {
        if capability == Capability::Pointer && self.pointer.is_none() {
            let pointer = self.seat_state.get_pointer(qh, &seat).expect("Failed to create pointer");
            self.pointer = Some(pointer);
        }
    }

    fn remove_capability(
        &mut self,
        _conn: &Connection,
        _: &QueueHandle<Self>,
        _: wl_seat::WlSeat,
        capability: Capability,
    ) {
        if capability == Capability::Pointer && self.pointer.is_some() {
            self.pointer.take().unwrap().release();
        }
    }

    fn remove_seat(&mut self, _: &Connection, _: &QueueHandle<Self>, _: wl_seat::WlSeat) {
        // Not needed for our widget
    }
}

enum FrameRequest {
    Redraw,
}

impl<T: 'static + Default + Send> Dispatch<wl_callback::WlCallback, FrameRequest> for WidgetState<T> {
    fn event(
        widget_state: &mut Self,
        _cb: &wl_callback::WlCallback,
        event: wl_callback::Event,
        data: &FrameRequest,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
    ) {
        if let wl_callback::Event::Done { .. } = event {
            match data {
                FrameRequest::Redraw => {
                    widget_state.draw();
                    {
                        let (lock, _) = widget_state.shared_widget.as_ref();
                        let mut shared_widget = lock.lock().unwrap();
                        shared_widget.frame_asked = false;
                    }
                }
            }
        }
    }
}

impl<T> ShmHandler for WidgetState<T> {
    fn shm_state(&mut self) -> &mut Shm {
        &mut self.shm
    }
}

impl<T: 'static + Default + Send> ProvidesRegistryState for WidgetState<T> {
    fn registry(&mut self) -> &mut RegistryState {
        &mut self.registry_state
    }
    registry_handlers![OutputState, SeatState];
}

delegate_compositor!(@<T: 'static + Default + Send> WidgetState<T>);
delegate_output!(@<T: 'static + Default + Send> WidgetState<T>);
delegate_shm!(@<T> WidgetState<T>);
delegate_seat!(@<T: 'static + Default + Send> WidgetState<T>);
delegate_pointer!(@<T: 'static + Default + Send> WidgetState<T>);
delegate_layer!(@<T: 'static + Default + Send> WidgetState<T>);
delegate_registry!(@<T: 'static + Default + Send> WidgetState<T>);
