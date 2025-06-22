use crate::winit::{KeyCode, WindowAttributes};

#[derive(Debug)]
pub struct AppConfigs {
    pub window_attributes: WindowAttributes,
    pub updates_per_second: u32,
    pub key_play: Option<KeyCode>,
    pub key_update_once: Option<KeyCode>,
    pub key_grid: Option<KeyCode>,
}

impl Default for AppConfigs {
    #[inline]
    fn default() -> Self {
        Self {
            window_attributes: WindowAttributes::default(),
            updates_per_second: 60,
            key_play: Some(KeyCode::Space),
            key_update_once: Some(KeyCode::Enter),
            key_grid: Some(KeyCode::KeyG),
        }
    }
}

impl AppConfigs {
    #[inline]
    pub fn new() -> Self {
        Self::default()
    }

    #[inline]
    pub fn window_attributes(self, window_attributes: WindowAttributes) -> Self {
        Self {
            window_attributes,
            ..self
        }
    }

    #[inline]
    pub fn updates_per_second(self, updates_per_second: u32) -> Self {
        Self {
            updates_per_second,
            ..self
        }
    }

    #[inline]
    pub fn key_play(self, key_play: Option<KeyCode>) -> Self {
        Self { key_play, ..self }
    }

    #[inline]
    pub fn key_update_once(self, key_update_once: Option<KeyCode>) -> Self {
        Self {
            key_update_once,
            ..self
        }
    }

    #[inline]
    pub fn key_grid(self, key_grid: Option<KeyCode>) -> Self {
        Self { key_grid, ..self }
    }
}
