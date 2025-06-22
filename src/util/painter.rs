use crate::{MouseEvent, World, WorldImage, util::is_pressed};
use std::collections::BTreeMap;
use winit::{
    event::{KeyEvent, MouseButton},
    keyboard::KeyCode,
};

pub struct WithPainter<W, Ink, F> {
    world: W,

    // Configs
    palette: BTreeMap<KeyCode, Ink>,
    paint_fn: F,

    // Painter state
    selected: Option<Ink>,
    mouse_pos_prev: Option<(u32, u32)>,
    mouse_pos: Option<(u32, u32)>,
    is_painting: bool,
}

impl<W: World, Ink, F> WithPainter<W, Ink, F>
where
    F: Fn(&mut W, u32, u32, Ink, &mut WorldImage),
{
    #[inline]
    pub fn new<P>(world: W, palette: P, paint_fn: F, selected: Option<Ink>) -> Self
    where
        P: IntoIterator<Item = (KeyCode, Ink)>,
    {
        Self {
            world,
            palette: palette.into_iter().collect(),
            paint_fn,
            selected,
            mouse_pos_prev: None,
            mouse_pos: None,
            is_painting: false,
        }
    }
}

impl<W, Ink, F> WithPainter<W, Ink, F>
where
    W: World,
    Ink: Clone,
    F: Fn(&mut W, u32, u32, Ink, &mut WorldImage),
{
    fn draw(&mut self, image: &mut WorldImage) {
        if self.is_painting {
            if let Some(ref ink) = self.selected {
                if let Some((x0, y0)) = self.mouse_pos_prev {
                    if let Some((x1, y1)) = self.mouse_pos {
                        for (x, y) in line_drawing::Bresenham::new(
                            (x0 as i32, y0 as i32),
                            (x1 as i32, y1 as i32),
                        ) {
                            (self.paint_fn)(
                                &mut self.world,
                                x as u32,
                                y as u32,
                                ink.clone(),
                                image,
                            );
                        }
                    }
                }
            }
        }
    }
}

impl<W, Ink, F> World for WithPainter<W, Ink, F>
where
    W: World,
    Ink: Clone,
    F: Fn(&mut W, u32, u32, Ink, &mut WorldImage),
{
    #[inline]
    fn init_image(&mut self) -> WorldImage {
        self.world.init_image()
    }

    #[inline]
    fn update(&mut self, image: &mut WorldImage) {
        self.world.update(image);
    }

    #[inline]
    fn keyboard_input(&mut self, event: KeyEvent, image: &mut WorldImage) {
        for (key, ink) in &self.palette {
            if is_pressed(&event, *key) {
                self.selected = Some(ink.clone());
            }
        }
        self.world.keyboard_input(event, image);
    }

    #[inline]
    fn mouse_input(&mut self, event: MouseEvent, image: &mut WorldImage) {
        let MouseEvent { state, button, .. } = event;

        if button == MouseButton::Left {
            self.is_painting = state.is_pressed();
        }
        self.draw(image);

        self.world.mouse_input(event, image);
    }

    fn cursor_moved(&mut self, pos: Option<(u32, u32)>, image: &mut WorldImage) {
        self.mouse_pos_prev = self.mouse_pos;
        self.mouse_pos = pos;
        if self.mouse_pos_prev.is_none() {
            self.mouse_pos_prev = self.mouse_pos;
        }
        self.draw(image);

        self.world.cursor_moved(pos, image);
    }
}

pub trait WithPainterExt: World {
    #[inline]
    fn with_painter<P, F, Ink>(self, palette: P, paint_fn: F, selected: Option<Ink>) -> impl World
    where
        P: IntoIterator<Item = (KeyCode, Ink)>,
        Ink: Clone,
        F: Fn(&mut Self, u32, u32, Ink, &mut WorldImage),
        Self: Sized,
    {
        WithPainter::new(self, palette, paint_fn, selected)
    }
}
impl<W: World> WithPainterExt for W {}

// pub trait WithPainterExtGrid: World + WorldGrid2d<Cell: Clone> {
//     #[inline]
//     fn with_painter_grid<P>(
//         self,
//         palette: P,
//         selected: Option<Self::Cell>,
//     ) -> impl World
//     where
//         P: IntoIterator<Item = (KeyCode, Self::Cell)>,
//         Self: Sized,
//     {
//         WithPainter::new(self, palette, |world, x, y, cell, image| {
//             if let Some(dst) = world.get_cell_mut(x, y) {

//             }
//         })
//     }
// }
