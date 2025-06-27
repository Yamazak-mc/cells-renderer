use crate::{MouseEvent, World, WorldImage, util::is_pressed};
use std::collections::BTreeMap;
use winit::{
    event::{KeyEvent, MouseButton},
    keyboard::KeyCode,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PainterDescriptor<Ink, F> {
    pub palette: BTreeMap<KeyCode, Ink>,
    pub paint_fn: Option<F>,
    pub selected: Option<Ink>,
    pub key_fill: Option<KeyCode>,
    pub key_fill_random: Option<KeyCode>,
    pub key_brush_expand: Option<KeyCode>,
    pub key_brush_shrink: Option<KeyCode>,
}

impl<Ink, F> Default for PainterDescriptor<Ink, F> {
    #[inline]
    fn default() -> Self {
        Self {
            palette: BTreeMap::default(),
            paint_fn: None,
            selected: None,
            key_fill: Some(KeyCode::KeyF),
            key_fill_random: Some(KeyCode::KeyR),
            key_brush_expand: Some(KeyCode::ArrowUp),
            key_brush_shrink: Some(KeyCode::ArrowDown),
        }
    }
}

impl<Ink, F> PainterDescriptor<Ink, F> {
    #[inline]
    pub fn palette(self, palette: BTreeMap<KeyCode, Ink>) -> Self {
        Self { palette, ..self }
    }

    #[inline]
    pub fn paint_fn(self, paint_fn: Option<F>) -> Self {
        Self { paint_fn, ..self }
    }

    #[inline]
    pub fn selected(self, selected: Option<Ink>) -> Self {
        Self { selected, ..self }
    }

    #[inline]
    pub fn key_fill(self, key_fill: Option<KeyCode>) -> Self {
        Self { key_fill, ..self }
    }

    #[inline]
    pub fn key_fill_random(self, key_fill_random: Option<KeyCode>) -> Self {
        Self {
            key_fill_random,
            ..self
        }
    }
}

pub struct WithPainter<W, Ink, F> {
    world: W,

    // Configs
    desc: PainterDescriptor<Ink, F>,

    mouse_pos_prev: Option<(u32, u32)>,
    mouse_pos: Option<(u32, u32)>,
    is_painting: bool,
    brush_size: u32,
}

impl<W, Ink, F> WithPainter<W, Ink, F> {
    const BRUSH_SIZE_MAX: u32 = 10;
}

impl<W: World, Ink, F> WithPainter<W, Ink, F>
where
    F: Fn(&mut W, u32, u32, Ink, &mut WorldImage),
{
    #[inline]
    pub fn new(world: W, desc: PainterDescriptor<Ink, F>) -> Self {
        Self {
            world,
            desc,
            mouse_pos_prev: None,
            mouse_pos: None,
            is_painting: false,
            brush_size: 0,
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
        if self.desc.paint_fn.is_none() {
            return;
        }
        if self.is_painting {
            if let Some(ref ink) = self.desc.selected {
                if let Some((x0, y0)) = self.mouse_pos_prev {
                    if let Some((x1, y1)) = self.mouse_pos {
                        let ink = ink.clone();
                        for (x, y) in line_drawing::Bresenham::new(
                            (x0 as i32, y0 as i32),
                            (x1 as i32, y1 as i32),
                        ) {
                            self.draw_at(image, x as u32, y as u32, &ink);
                        }
                    }
                }
            }
        }
    }

    fn draw_at(&mut self, image: &mut WorldImage, x: u32, y: u32, ink: &Ink) {
        let width = image.width();
        let height = image.height();

        let b = self.brush_size as i32;

        for oy in -b..=b {
            let Some(y_) = y.checked_add_signed(oy) else {
                continue;
            };
            if y_ >= height {
                continue;
            }
            for ox in -b..=b {
                let Some(x_) = x.checked_add_signed(ox) else {
                    continue;
                };
                if x_ >= width {
                    continue;
                }
                (self.desc.paint_fn.as_mut().unwrap())(&mut self.world, x_, y_, ink.clone(), image);
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
        for (key, ink) in &self.desc.palette {
            if is_pressed(&event, *key) {
                self.desc.selected = Some(ink.clone());
            }
        }
        if let Some(key_brush_expand) = self.desc.key_brush_expand {
            if is_pressed(&event, key_brush_expand) && self.brush_size < Self::BRUSH_SIZE_MAX {
                self.brush_size += 1;
            }
        }
        if let Some(key_brush_shrink) = self.desc.key_brush_shrink {
            if is_pressed(&event, key_brush_shrink) {
                self.brush_size = self.brush_size.checked_sub(1).unwrap_or_default();
            }
        }
        if self.desc.paint_fn.is_some() {
            if let Some(key_fill) = self.desc.key_fill {
                if is_pressed(&event, key_fill) {
                    if let Some(ref ink) = self.desc.selected {
                        for y in 0..image.height() {
                            for x in 0..image.width() {
                                (self.desc.paint_fn.as_mut().unwrap())(
                                    &mut self.world,
                                    x,
                                    y,
                                    ink.clone(),
                                    image,
                                );
                            }
                        }
                    }
                }
            }
            if let Some(key_fill_random) = self.desc.key_fill_random {
                if is_pressed(&event, key_fill_random) && !self.desc.palette.is_empty() {
                    use rand::seq::IteratorRandom;
                    let mut rng = rand::rng();
                    for y in 0..image.height() {
                        for x in 0..image.width() {
                            (self.desc.paint_fn.as_mut().unwrap())(
                                &mut self.world,
                                x,
                                y,
                                self.desc.palette.values().choose(&mut rng).unwrap().clone(),
                                image,
                            );
                        }
                    }
                }
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
    fn with_painter<F, Ink>(self, desc: PainterDescriptor<Ink, F>) -> impl World
    where
        Ink: Clone,
        F: Fn(&mut Self, u32, u32, Ink, &mut WorldImage),
        Self: Sized,
    {
        WithPainter::new(self, desc)
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
