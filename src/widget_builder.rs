use smithay_client_toolkit::{
    compositor::CompositorState,
    output::OutputState,
    registry::RegistryState,
    seat::{SeatState, pointer::cursor_shape::CursorShapeManager,},
    shell::{wlr_layer::{Layer, LayerShell}},
    shm::Shm,
};
use std::{sync::{Arc, Condvar, Mutex, mpsc::{Receiver, Sender, channel}}, thread};
use wayland_client::{
    Connection, QueueHandle, globals::registry_queue_init,
};
use crate::{WidgetSize, WidgetPosition, Widget, WidgetState, WidgetAnchor, Margin, SizeUnit, SharedWidget, WidgetEvent, MouseHandler};

pub struct WidgetBuilder {
    size: WidgetSize,
    position: WidgetPosition,
    name: String,
    layer: Layer,
}

impl WidgetBuilder {
    pub fn new(size: WidgetSize, position: WidgetPosition, name: String) -> Self {
        WidgetBuilder {
            size,
            position,
            name,
            layer: Layer::Background,
        }
    }

    pub fn at_layer(mut self, layer: Layer) -> Self {
        self.layer = layer;
        self
    }

    pub fn build<T: 'static + Default + Send>(self) -> Widget<T> {
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
        let cursor_shape_manager = CursorShapeManager::bind(&globals, &qh).expect("cursor manager is not available");

        // We use wl_shm to allow software rendering to a buffer we share with the compositor process.
        let shm = Shm::bind(&globals, &qh).expect("wl_shm is not available");

        let (widget_anchor, margin) = match self.position {
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
                force_redraw: false,
                wl_surface: None,
                conn,

                mouse_handler: MouseHandler::default(),
                width: None,
                height: None,
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
            
            layer: None,
            cursor_shape_manager,
            pointer: None,

            widget_size: self.size,
            widget_name: self.name,
            widget_layer: self.layer,
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
}