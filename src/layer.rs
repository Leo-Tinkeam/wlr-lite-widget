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
use wayland_client::{
    Connection, QueueHandle, globals::{registry_queue_init}, protocol::{wl_output, wl_pointer, wl_seat, wl_shm, wl_surface}
};
use crate::{WidgetDetails, WidgetSize};

pub struct Widget {
    registry_state: RegistryState,
    seat_state: SeatState,
    output_state: OutputState,
    shm: Shm,
    compositor: CompositorState,
    layer_shell: LayerShell,

    exit: bool,
    need_redraw: bool,
    pool: Option<SlotPool>,
    width: Option<u32>,
    height: Option<u32>,
    clicked: bool,
    layer: Option<LayerSurface>,
    pointer: Option<wl_pointer::WlPointer>,

    render: fn(&mut [u8], u32, u32, bool),
    widget_size: WidgetSize,
    widget_name: String,
    widget_layer: Layer,
    widget_anchor: Option<Anchor>,
}

impl Widget {
    // TODO: Il faut la position de la même façon que size -> Renommer WidgetSize en WidgetBounds
    pub fn new(size: WidgetSize, name: String, details: Option<WidgetDetails>, render: fn(&mut [u8], u32, u32, bool)) {
        // Connecting to the compositor (server)
        let conn = Connection::connect_to_env().unwrap();

        // Enumerate the list of globals to get the protocols the server implements.
        let (globals, mut event_queue) = registry_queue_init(&conn).unwrap();
        let qh = event_queue.handle();

        // The compositor (not to be confused with the server which is commonly called the compositor) allows
        // configuring surfaces to be presented.
        let compositor = CompositorState::bind(&globals, &qh).expect("wl_compositor is not available");

        // This app uses the wlr layer shell, which may not be available with every compositor.
        let layer_shell = LayerShell::bind(&globals, &qh).expect("layer shell is not available");

        // We use wl_shm to allow software rendering to a buffer we share with the compositor process.
        let shm = Shm::bind(&globals, &qh).expect("wl_shm is not available");

        let widget_details = details.unwrap_or_default();
        let mut widget = Widget {
            // Seats and outputs may be hotplugged at runtime, therefore we need to setup a registry state to
            // listen for seats and outputs.
            registry_state: RegistryState::new(&globals),
            seat_state: SeatState::new(&globals, &qh),
            output_state: OutputState::new(&globals, &qh),
            shm,
            compositor: compositor,
            layer_shell: layer_shell,

            exit: false,
            need_redraw: true,
            pool: None,
            width: None,
            height: None,
            clicked: false,
            layer: None,
            pointer: None,

            render: render,
            widget_size: size,
            widget_name: name,
            widget_layer: widget_details.layer.unwrap_or(Layer::Background),
            widget_anchor: widget_details.anchor,
        };

        // We don't draw immediately, the configure will notify us when to first draw.
        loop {
            event_queue.blocking_dispatch(&mut widget).unwrap();
            if widget.exit {
                break;
            }
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

        // Render with the user render function
        (self.render)(canvas, width, height, self.clicked);

        // Damage the entire window
        // Here we should damage only concerned area
        layer.wl_surface().damage_buffer(0, 0, width as i32, height as i32);

        // Attach and commit to present.
        buffer.attach_to(layer.wl_surface()).expect("buffer attach");
        layer.commit();
    }
}

impl CompositorHandler for Widget {
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

impl OutputHandler for Widget {
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
                // A layer surface is created from a surface.
                let surface = self.compositor.create_surface(&qh);

                // And then we create our layer shell.
                let new_layer =
                    self.layer_shell.create_layer_surface(&qh, surface, self.widget_layer, Some(self.widget_name.clone()), Some(&output));
                
                let (screen_width, screen_height) = (info.logical_size.unwrap().0, info.logical_size.unwrap().1);
                (self.width, self.height) = self.widget_size.get_dimension(screen_width as u32, screen_height as u32);

                // Configure the layer surface with anchor on screen and desired size
                if let Some(anchor) = self.widget_anchor {
                    new_layer.set_anchor(anchor);
                }
                new_layer.set_size(self.width.unwrap(), self.height.unwrap());

                // In order for the layer surface to be mapped, we need to perform an initial commit with no attached\
                // buffer. For more info, see WaylandSurface::commit
                // The compositor will respond with an initial configure that we can then use to present to the layer
                // surface with the correct options.
                new_layer.commit();

                self.pool = Some(SlotPool::new((self.width.unwrap() * self.height.unwrap() * 4).try_into().expect("Too large dimension"), &self.shm).expect("Failed to create pool"));
                self.layer = Some(new_layer);
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
            (info.logical_size.unwrap().0, info.logical_size.unwrap().1)
        } else {
            return;
        };
        
        (self.width, self.height) = self.widget_size.get_dimension(screen_width as u32, screen_height as u32);
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

impl LayerShellHandler for Widget {
    fn closed(&mut self, _conn: &Connection, _qh: &QueueHandle<Self>, _layer: &LayerSurface) {
        self.exit = true;
    }

    fn configure(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _layer: &LayerSurface,
        _configure: LayerSurfaceConfigure,
        _serial: u32,
    ) {
        // Initiate the first draw.
        if self.need_redraw {
            self.need_redraw = false;
            self.draw();
        }
    }
}

impl SeatHandler for Widget {
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

impl PointerHandler for Widget {
    fn pointer_frame(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
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
                    println!("Press {:x} @ {:?}", button, event.position);
                    self.clicked = !self.clicked;
                    self.draw();
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

impl ShmHandler for Widget {
    fn shm_state(&mut self) -> &mut Shm {
        &mut self.shm
    }
}

impl ProvidesRegistryState for Widget {
    fn registry(&mut self) -> &mut RegistryState {
        &mut self.registry_state
    }
    registry_handlers![OutputState, SeatState];
}

delegate_compositor!(Widget);
delegate_output!(Widget);
delegate_shm!(Widget);
delegate_seat!(Widget);
delegate_pointer!(Widget);
delegate_layer!(Widget);
delegate_registry!(Widget);
