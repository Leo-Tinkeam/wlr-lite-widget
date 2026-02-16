use std::sync::atomic::{AtomicI32, Ordering};
use crate::{WidgetPosition, WidgetSize};

static NEXT_SURFACE_ID: AtomicI32 = AtomicI32::new(1);

pub struct Surface {
    pub(crate) id: i32,
    pub(crate) size: WidgetSize,
    pub(crate) position: WidgetPosition,
    pub(crate) render: fn(&mut [u8], u32, u32, bool), // TODO: Help user to create these for exemple fill_color() and a custom type for advanced shapes
}

impl Surface {
    pub fn new(size: WidgetSize, position: WidgetPosition, render: fn(&mut [u8], u32, u32, bool)) -> Self {
        let id = NEXT_SURFACE_ID.fetch_add(1, Ordering::Relaxed);

        Surface {
            id,
            size,
            position,
            render,
        }
    }

    pub fn edit_size(&mut self, new_size: WidgetSize) {
        self.size = new_size;

        // Call a forced re-draw
    }

    pub fn edit_position(&mut self, new_position: WidgetPosition) {
        self.position = new_position;

        // Call a forced re-draw
    }

    pub fn edit_render(&mut self, new_render: fn(&mut [u8], u32, u32, bool)) {
        self.render = new_render;

        // Call a forced re-draw
    }

    pub fn to_front_of(&mut self, other_surface: &mut Surface) {
        if self.id < other_surface.id {
            let temp_id = self.id;
            self.id = other_surface.id;
            other_surface.id = temp_id;

            // Call a forced re-draw on both Surface
        }
    }

    // We should be able to force a re-draw of the widget

    // We should be able to animate this on "hover" (maybe render with a 0-1 float)
}