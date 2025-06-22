pub mod winit {
    pub use winit::{
        event::KeyEvent,
        event::{ElementState, MouseButton},
        keyboard::KeyCode,
        window::WindowAttributes,
    };
}

pub mod image;
pub use image::WorldImage;

pub mod configs;
pub use configs::AppConfigs;

pub mod mouse_event;
pub use mouse_event::MouseEvent;

pub mod world;
pub use world::World;

pub mod app;
pub use app::App;

pub mod util;

pub mod prelude {
    pub use crate::{App, AppConfigs, MouseEvent, World as WorldTrait, WorldImage, winit::*};
}
