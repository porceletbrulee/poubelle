use std::collections::HashMap;
use std::iter::repeat_with;
use std::panic::catch_unwind;

use macroquad::prelude::*;

type Elevation = i32;
type ElevationDelta = i32;

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
struct Coord {
	x: usize,
	y: usize,
}

impl From<&(usize, usize)> for Coord {
	fn from(input: &(usize, usize)) -> Self {
		Coord{x: input.0, y: input.1}
	}
}

#[derive(Debug)]
struct Grid<T>{
	tile_array: Box<[Option<T>]>,
	width: usize,
	height: usize,
}

#[repr(u8)]
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
enum PlaneDir {
	North = 0u8,
	East = 1u8,
	South = 2u8,
	West = 3u8,
}

impl TryFrom<u8> for PlaneDir {
	type Error = &'static str;

	fn try_from(val: u8) -> Result<Self, Self::Error> {
		match val {
			0 => Ok(Self::North),
			1 => Ok(Self::East),
			2 => Ok(Self::South),
			3 => Ok(Self::West),
			_ => Err("bad value for PlaneDir")
		}
	}
}

impl PlaneDir {
	// gives x s.t. self.rotate(x) == other
	pub fn rotate_diff(&self, other: PlaneDir) -> i8 {
		(other as i8) - (*self as i8)
	}

	pub fn rotate(&self, n: i8) -> PlaneDir {
		PlaneDir::try_from(((*self as i8) + (n as i8)).rem_euclid(4) as u8).expect("unreachable")
	}

	pub fn clockwise(&self) -> PlaneDir {
		self.rotate(1)
	}

	pub fn anticlockwise(&self) -> PlaneDir {
		self.rotate(-1)
	}
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
struct DirInfo {
	elevation_delta: ElevationDelta,
	enterable: bool,
}

enum TileDirTemplate {
	Floor,
	SingleWall,
	Impasse,
	Stair,
	Ramp,
}

enum TileType {
	Sidewalk,
	Freewalk,
	Impasse,
	Stair,
	Ramp,
	Road,
}

#[derive(Debug)]
struct Tile {
	facing: PlaneDir,
	dir_infos: HashMap<PlaneDir, DirInfo>,
}

impl Tile {
	pub const DEFAULT_FACING: PlaneDir = PlaneDir::North;

	pub fn new() -> Tile {
		Tile{facing: Self::DEFAULT_FACING, dir_infos: HashMap::new()}
	}

	pub fn get(&self, dir: &PlaneDir) -> DirInfo {
		match self.dir_infos.get(dir) {
			Some(i) => *i,
			None => DirInfo{elevation_delta: 0, enterable: true},
		}
	}
}

impl<T> Grid<T> {
	pub fn new(width: usize, height: usize) -> Grid<T> {
		let v = repeat_with(|| None).take(width * height).collect::<Vec<_>>();
		Grid::<T>{
			tile_array: v.into_boxed_slice(),
			width: width,
			height: height,
		}
	}

	fn maybe_coord_to_index(&self, coord: &Coord) -> Result<usize, ()> {
		let i = coord.x + (coord.y * self.width);
		if coord.x >= self.width ||
		   coord.y >= self.height {
			return Err(());
		}
		return Ok(i);
	}

	fn coord_to_index(&self, coord: &Coord) -> usize {
		match self.maybe_coord_to_index(coord) {
			Ok(i) => i,
			Err(_) => panic!("coord {:?} width {} height {}", coord, self.width, self.height),
		}
	}

	pub fn get(&self, coord: &Coord) -> &Option<T> {
		&self.tile_array[self.coord_to_index(coord)]
	}

	pub fn remove(&mut self, coord: &Coord) {
		self.tile_array[self.coord_to_index(coord)] = None
	}

	pub fn add(&mut self, coord: &Coord, t: T) {
		self.tile_array[self.coord_to_index(coord)] = Some(t)
	}
}

struct TileMap {
	layers: HashMap<Elevation, Grid<Tile>>,
}

#[cfg(test)]
mod test {
	use super::*;

	#[test]
	pub fn test_grid() {
		let g = Grid::<Tile>::new(3, 4);
		let coords = vec![(1, 7), (3, 2), (8, 9)];
		for c in &coords {
			let coord = Coord::from(c);
			let result = catch_unwind(|| {
				g.get(&coord);
			});
			assert!(result.is_err());
		}
	}

	#[test]
	pub fn test_add_and_get() {
		let mut g = Grid::<u8>::new(2, 3);
		let coord = Coord{x: 1, y: 0};
		let val = 7u8;
		g.add(&coord, val);
		match g.get(&coord) {
			Some(i) => assert_eq!(*i, val),
			None => panic!("expected {} at {:?}", val, coord),
		};
	}

	#[test]
	pub fn test_rotate() {
		assert_eq!(PlaneDir::North.clockwise(), PlaneDir::East);
		assert_eq!(PlaneDir::East.clockwise(), PlaneDir::South);
		assert_eq!(PlaneDir::South.clockwise(), PlaneDir::West);
		assert_eq!(PlaneDir::West.clockwise(), PlaneDir::North);
		assert_eq!(PlaneDir::North.anticlockwise(), PlaneDir::West);
		assert_eq!(PlaneDir::East.anticlockwise(), PlaneDir::North);
		assert_eq!(PlaneDir::South.anticlockwise(), PlaneDir::East);
		assert_eq!(PlaneDir::West.anticlockwise(), PlaneDir::South);
		assert_eq!(PlaneDir::North.rotate_diff(PlaneDir::South), 2);
		assert_eq!(PlaneDir::East.rotate_diff(PlaneDir::North), -1);

		let diff = PlaneDir::West.rotate_diff(PlaneDir::South);
		assert_eq!(PlaneDir::West.rotate(diff), PlaneDir::South);
	}
}

#[derive(Debug)]
struct Display {
	origin: (f32, f32),
	dim: (f32, f32),
	tile_side_len: f32,
	grid_size: (usize, usize),
}

impl Display {
	const TILE_MARGIN: f32 = 1.0;

	pub fn new(swidth: f32, sheight: f32, x_tiles: usize, y_tiles: usize) -> Display {
		let display_width = swidth * 9.0 / 10.0;
		let display_height = sheight * 9.0 / 10.0;
		return Display{
			origin: (swidth / 20.0, sheight / 20.0),
			dim: (display_width, display_height),
			// tile should be square, pick width
			tile_side_len: (display_width / x_tiles as f32),
			grid_size: (x_tiles, y_tiles),
		};
	}

	pub fn draw_empty_tile(&self, coord: &Coord, color: &Color) {
		draw_rectangle(
			self.origin.0 + ((coord.x as f32) * self.tile_side_len) + Self::TILE_MARGIN,
			self.origin.1 + ((coord.y as f32) * self.tile_side_len) + Self::TILE_MARGIN,
			self.tile_side_len - Self::TILE_MARGIN * 2.0,
			self.tile_side_len - Self::TILE_MARGIN * 2.0,
			*color);
	}
}

#[macroquad::main("BasicShapes")]
async fn main() {
	let x_tiles = 48;
	let y_tiles = 32;
	let mut g = Grid::<Tile>::new(x_tiles, y_tiles);

    loop {
        clear_background(BLACK);

		let swidth = screen_width();
		let sheight = screen_height();

		let display = Display::new(swidth, sheight, x_tiles, y_tiles);
        draw_rectangle(display.origin.0, display.origin.1, display.dim.0, display.dim.1, DARKGRAY);

		for x in 0..x_tiles {
			for y in 0..y_tiles {
				display.draw_empty_tile(&Coord{x: x, y: y}, &Color{r: 220.0, g: 220.0, b: 220.0, a: 0.75});
			}
		}

        //draw_line(40.0, 40.0, 100.0, 200.0, 15.0, BLUE);
        //draw_circle(screen_width() - 30.0, screen_height() - 30.0, 15.0, YELLOW);
        //draw_text("IT WORKS!", 20.0, 20.0, 30.0, DARKGRAY);

        next_frame().await
    }
}
