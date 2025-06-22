use cells_renderer::{prelude::*, util::*};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
enum Cell {
    #[default]
    Dead,
    Alive,
}

impl Cell {
    fn new(is_alive: bool) -> Self {
        if is_alive { Self::Alive } else { Self::Dead }
    }

    fn color(&self) -> [u8; 4] {
        match self {
            Self::Dead => [0, 0, 0, 255],
            Self::Alive => [255, 255, 255, 255],
        }
    }

    fn is_alive(&self) -> bool {
        matches!(self, Self::Alive)
    }
}

struct World {
    width: u32,
    height: u32,
    cells: Vec<Cell>,
    cells_temp: Vec<Cell>,
}

impl World {
    fn new(width: u32, height: u32) -> Self {
        let cells = vec![Cell::Dead; width as usize * height as usize];
        let cells_temp = cells.clone();
        Self {
            width,
            height,
            cells,
            cells_temp,
        }
    }

    fn calc_index(&self, x: u32, y: u32) -> usize {
        (x + y * self.width) as usize
    }

    fn update_image(&self, image: &mut WorldImage) {
        debug_assert_eq!(image.width(), self.width);
        debug_assert_eq!(image.height(), self.height);

        for (src, dst) in self.cells.iter().zip(image.buf_mut().chunks_exact_mut(4)) {
            dst.copy_from_slice(&src.color());
        }
    }

    fn update_cell(&mut self, x: u32, y: u32, image: &mut WorldImage) {
        let x0 = (x + self.width - 1) % self.width;
        let x1 = (x + 1) % self.width;
        let y0 = (y + self.height - 1) % self.height;
        let y1 = (y + 1) % self.height;

        let idx = self.calc_index(x, y);
        let is_alive = self.cells[idx].is_alive();
        let n_alive = [
            (x0, y0),
            (x, y0),
            (x1, y0),
            (x0, y),
            (x1, y),
            (x0, y1),
            (x, y1),
            (x1, y1),
        ]
        .iter()
        .filter(|(x, y)| self.cells[self.calc_index(*x, *y)].is_alive())
        .count();
        let is_alive_out = (n_alive == 3) || (is_alive && n_alive == 2);
        let cell_out = Cell::new(is_alive_out);
        self.cells_temp[idx] = cell_out;
        if is_alive_out != is_alive {
            image
                .get_mut(x, y)
                .unwrap()
                .copy_from_slice(&cell_out.color());
        }
    }
}

impl WorldTrait for World {
    fn init_image(&mut self) -> WorldImage {
        let mut image = WorldImage::new(self.width, self.height);
        self.update_image(&mut image);
        image
    }

    fn update(&mut self, image: &mut WorldImage) {
        for y in 0..self.height {
            for x in 0..self.width {
                self.update_cell(x, y, image);
            }
        }
        std::mem::swap(&mut self.cells, &mut self.cells_temp);
    }
}

fn main() {
    App::new(
        AppConfigs::default(),
        World::new(32, 32).with_painter(
            [
                (KeyCode::Digit0, Cell::Dead),
                (KeyCode::Digit1, Cell::Alive),
            ],
            |world, x, y, cell, image| {
                let idx = world.calc_index(x, y);
                world.cells[idx] = cell;
                image.get_mut(x, y).unwrap().copy_from_slice(&cell.color());
            },
            Some(Cell::Alive),
        ),
    )
    .run()
    .unwrap();
}
