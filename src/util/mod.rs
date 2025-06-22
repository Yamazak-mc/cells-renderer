use winit::{
    event::KeyEvent,
    keyboard::{KeyCode, PhysicalKey},
};

pub mod painter;
pub use painter::{WithPainter, WithPainterExt};

pub(crate) fn is_pressed(event: &KeyEvent, key: KeyCode) -> bool {
    event.state.is_pressed() && event.physical_key == PhysicalKey::Code(key)
}
