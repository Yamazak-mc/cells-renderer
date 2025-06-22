use crate::{MouseEvent, WorldImage, winit::KeyEvent};

pub trait World {
    fn init_image(&mut self) -> WorldImage;

    #[inline]
    fn update(&mut self, image: &mut WorldImage) {
        let _ = image;
    }

    #[inline]
    fn keyboard_input(&mut self, event: KeyEvent, image: &mut WorldImage) {
        let _ = (event, image);
    }

    #[inline]
    fn mouse_input(&mut self, event: MouseEvent, image: &mut WorldImage) {
        let _ = (event, image);
    }

    #[inline]
    fn cursor_moved(&mut self, pos: Option<(u32, u32)>, image: &mut WorldImage) {
        let _ = (pos, image);
    }
}
