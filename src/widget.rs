use smithay_client_toolkit::{
    compositor::{CompositorHandler, CompositorState},
    delegate_compositor, delegate_layer, delegate_output,
    delegate_pointer, delegate_registry, delegate_seat,
    delegate_shm,
    output::{OutputHandler, OutputState},
    registry::{ProvidesRegistryState, RegistryState},
    registry_handlers,
    seat::{
        Capability, SeatHandler, SeatState,
        pointer::{PointerEvent, PointerEventKind, PointerHandler}
    },
    shell::{
        WaylandSurface,
        wlr_layer::{
            Anchor, Layer, LayerShell,
            LayerShellHandler, LayerSurface,LayerSurfaceConfigure,
        },
    },
    shm::{Shm, ShmHandler, slot::SlotPool},
};
use std::{sync::{Arc, Condvar, Mutex, mpsc::{Receiver, Sender, channel}}, thread};
use wayland_client::{
    Connection, Dispatch, QueueHandle, globals::registry_queue_init, protocol::{wl_callback, wl_output, wl_pointer, wl_seat, wl_shm, wl_surface}
};
use crate::{SizeUnit, Surface, WidgetAnchor, Margin, WidgetPosition, WidgetSize};

#[derive(Clone)]
pub struct Widget<T> {
    shared_widget: Arc<(Mutex<SharedWidget<T>>, Condvar)>,
}

impl<T: 'static + Default + Send> Widget<T> {
    pub fn new(size: WidgetSize, position: WidgetPosition, name: String, layer: Option<Layer>) -> Self {
        WidgetState::create(size, position, name, layer)
    }

    pub fn add_surface(&mut self, mut surface: Surface<T>) {
        let mut shared_widget = self.shared_widget.0.lock().unwrap();
        surface.event_sender = Some(shared_widget.event_sender.clone());
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

    pub fn redraw(&self) {
        let shared_widget = self.shared_widget.0.lock().unwrap();
        if shared_widget.event_sender.send(WidgetEvent::Redraw).is_err() {
            println!("Error: redraw not sent");
        };
    }

    pub fn update_app_state(&self, new_state: T) {
        let mut shared_widget = self.shared_widget.0.lock().unwrap();
        shared_widget.app_state = new_state;
    }
}

struct SharedWidget<T> {
    exit: bool, 
    app_state: T,
    surfaces: Vec<Surface<T>>,
    //buttons: Vec<Button>, // TODO
    event_sender: Sender<WidgetEvent>,
    frame_asked: bool,
    wl_surface: Option<wl_surface::WlSurface>,
    conn: Connection,
}

impl<T: 'static + Default + Send> SharedWidget<T> {
    fn ask_redraw(&mut self, qh: &QueueHandle<WidgetState<T>>) {
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

struct WidgetState<T> {
    registry_state: RegistryState,
    seat_state: SeatState,
    output_state: OutputState,
    shm: Shm,
    compositor: CompositorState,
    layer_shell: LayerShell,

    need_redraw: bool,
    pool: Option<SlotPool>,
    width: Option<u32>,
    height: Option<u32>,
    layer: Option<LayerSurface>,
    pointer: Option<wl_pointer::WlPointer>,

    widget_size: WidgetSize,
    widget_name: String,
    widget_layer: Layer,
    widget_anchor: Option<Anchor>,
    margin: Margin,

    shared_widget: Arc<(Mutex<SharedWidget<T>>, Condvar)>,
}

impl<T: 'static + Default + Send> WidgetState<T> {
    fn create(size: WidgetSize, position: WidgetPosition, name: String, layer: Option<Layer>) -> Widget<T> {
        // Connecting to the compositor (server)
        let conn = Connection::connect_to_env().unwrap();

        // Enumerate the list of globals to get the protocols the server implements.
        let (globals, mut event_queue) = registry_queue_init(&conn).unwrap();
        let qh: QueueHandle<WidgetState<T>> = event_queue.handle();

        // The compositor (not to be confused with the server which is commonly called the compositor) allows
        // configuring surfaces to be presented.
        let compositor = CompositorState::bind(&globals, &qh).expect("wl_compositor is not available");

        // This app uses the wlr layer shell, which may not be available with every compositor.
        let layer_shell = LayerShell::bind(&globals, &qh).expect("layer shell is not available");

        // We use wl_shm to allow software rendering to a buffer we share with the compositor process.
        let shm = Shm::bind(&globals, &qh).expect("wl_shm is not available");

        let (widget_anchor, margin) = match position {
            WidgetPosition::Coordinates(x, y) => {(
                WidgetAnchor::TopLeft.into(),
                Margin {
                    top: y,
                    right: SizeUnit::Pixel(0),
                    bottom: SizeUnit::Pixel(0),
                    left: x,
                }
            )},
            WidgetPosition::Anchor(anchor, margin_temp) => {
                (
                    anchor.clone().into(),
                    margin_temp.unwrap_or_default().into_margin(anchor),
                )
            }
        };

        let (tx, rx): (Sender<WidgetEvent>, Receiver<WidgetEvent>) = channel();
        let shared_widget = Arc::new((
            Mutex::new(SharedWidget {
                exit: false,
                app_state: T::default(),
                surfaces: vec![],
                event_sender: tx,
                frame_asked: false,
                wl_surface: None,
                conn,
            }),
            Condvar::new(),
        ));

        let mut widget_state = WidgetState {
            // Seats and outputs may be hotplugged at runtime, therefore we need to setup a registry state to
            // listen for seats and outputs.
            registry_state: RegistryState::new(&globals),
            seat_state: SeatState::new(&globals, &qh),
            output_state: OutputState::new(&globals, &qh),
            shm,
            compositor: compositor,
            layer_shell: layer_shell,

            need_redraw: true,
            pool: None,
            width: None,
            height: None,
            
            layer: None,
            pointer: None,

            widget_size: size,
            widget_name: name,
            widget_layer: layer.unwrap_or(Layer::Background),
            widget_anchor,
            margin,

            shared_widget: Arc::clone(&shared_widget),
        };

        thread::spawn(move || {
            loop {
                event_queue.blocking_dispatch(&mut widget_state).unwrap();
                {
                    let shared_widget = widget_state.shared_widget.0.lock().unwrap();
                    if shared_widget.exit {
                        break;
                    }
                }
            }
        });

        let shared_widget_clone = Arc::clone(&shared_widget);
        thread::spawn(move || {
            while let Ok(event) = rx.recv() {
                match event {
                    WidgetEvent::Exit => return,
                    WidgetEvent::Redraw => {
                        shared_widget_clone.0.lock().unwrap().ask_redraw(&qh);
                    },
                }
            }
        });

        Widget {
            shared_widget: shared_widget,
        }
    }

    pub fn draw(&mut self) {
        let layer = match self.layer.clone() {
            None => return,
            Some(layer) => layer,
        };
        let width = self.width.unwrap();
        let height = self.height.unwrap();
        let stride = width as i32 * 4;
        let (buffer, canvas) = self.pool
            .as_mut()
            .unwrap()
            .create_buffer(width as i32, height as i32, stride, wl_shm::Format::Argb8888)
            .expect("Error while creating buffer");

        {
            // Protected access to shared_widget for the duration of this block
            let mut shared_widget = self.shared_widget.0.lock().unwrap();

            // Render with the user render function
            shared_widget.surfaces.sort_by_key(|s| s.id); // TODO: only call this when to_front_of is call on a Surface ?
            let len = shared_widget.surfaces.len();
            for i in 0..len {
                // TODO: surfaces without "need_redraw" do not need redraw if they are not above a redrawed surface
                let (surface_width, surface_height) = shared_widget.surfaces[i].size.get_dimension(width, height);
                (shared_widget.surfaces[i].render)(canvas, surface_width.unwrap(), surface_height.unwrap(), &mut shared_widget.app_state); // TODO: Use the position and size of the Surface

                let (x, y) = shared_widget.surfaces[i].position.get_coordinates(width, height, (surface_width.unwrap(), surface_height.unwrap()));
                layer.wl_surface().damage_buffer(x, y, surface_width.unwrap() as i32, surface_height.unwrap() as i32); // TODO: should damage (and redraw) only when something change (and save that we have applied that change into the Surface)
            }
        }

        // Attach and commit to present.
        buffer.attach_to(layer.wl_surface()).expect("buffer attach");
        layer.commit();
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
    ) {
        // TODO: Will be used for "hover" animations
    }

    fn surface_leave(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _surface: &wl_surface::WlSurface,
        _output: &wl_output::WlOutput,
    ) {
        // TODO: Will be used for "hover" animations
    }
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
                (self.width, self.height) = self.widget_size.get_dimension(screen_width, screen_height);
                new_layer.set_size(self.width.unwrap(), self.height.unwrap());

                // In order for the layer surface to be mapped, we need to perform an initial commit with no attached\
                // buffer. For more info, see WaylandSurface::commit
                // The compositor will respond with an initial configure that we can then use to present to the layer
                // surface with the correct options.
                new_layer.commit();

                self.pool = Some(SlotPool::new((self.width.unwrap() * self.height.unwrap() * 4).try_into().expect("Too large dimension"), &self.shm).expect("Failed to create pool"));
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
        (self.width, self.height) = self.widget_size.get_dimension(screen_width, screen_height);
        self.layer.as_mut().unwrap().set_size(self.width.unwrap(), self.height.unwrap());

        self.layer.as_mut().unwrap().commit();
        self.pool = Some(SlotPool::new((self.width.unwrap() * self.height.unwrap() * 4).try_into().expect("Too large dimension"), &self.shm).expect("Failed to create pool"));
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

impl<T: 'static + Default + Send> PointerHandler for WidgetState<T> {
    fn pointer_frame(
        &mut self,
        _conn: &Connection,
        qh: &QueueHandle<Self>,
        _pointer: &wl_pointer::WlPointer,
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
                Enter { .. } => {
                    println!("Pointer entered @{:?}", event.position);
                }
                Leave { .. } => {
                    println!("Pointer left");
                }
                Motion { .. } => {}
                Press { button, .. } => {
                    /* println!("Press {:x} @ {:?}", button, event.position);
                    self.clicked = !self.clicked;
                    {
                        let (lock, _) = self.shared_widget.as_ref();
                        let mut shared_widget = lock.lock().unwrap();
                        shared_widget.ask_redraw(qh);
                    } */
                   // TODO: Appeler un onClic
                }
                Release { button, .. } => {
                    println!("Release {:x} @ {:?}", button, event.position);
                }
                Axis { horizontal, vertical, .. } => {
                    println!("Scroll H:{horizontal:?}, V:{vertical:?}");
                }
            }
        }
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
