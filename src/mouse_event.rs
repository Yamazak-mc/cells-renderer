use crate::winit::{ElementState, MouseButton};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct MouseEvent {
    pub state: ElementState,
    pub button: MouseButton,
    pub pos: Option<(u32, u32)>,
}
