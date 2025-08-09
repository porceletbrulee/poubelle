use std::collections::HashMap;
use std::iter::repeat_with;

use macroquad::prelude::*;
use miniquad::conf::{Platform, WebGLVersion};

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
	use std::panic::catch_unwind;

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
	grid_rect: Rect,
	tile_side_len: f32,
	grid_size: (usize, usize),
}

impl Display {
	const TILE_MARGIN: f32 = 1.0;

	pub fn new(swidth: f32, sheight: f32, x_tiles: usize, y_tiles: usize) -> Display {
		let margin_fraction = 0.025;
		let margin = swidth * margin_fraction;
		let usable_fraction = 1.0 - (2.0 * margin_fraction);

		let grid_width = swidth * usable_fraction;
		return Display{
			grid_rect: Rect{x: margin, y: margin, w: grid_width, h: sheight * usable_fraction},
			// tile should be square, pick width
			tile_side_len: (grid_width / x_tiles as f32),
			grid_size: (x_tiles, y_tiles),
		};
	}

	pub fn draw_empty_tile(&self, coord: &Coord) {
		draw_rectangle(
			self.grid_rect.x + ((coord.x as f32) * self.tile_side_len) + Self::TILE_MARGIN,
			self.grid_rect.y + ((coord.y as f32) * self.tile_side_len) + Self::TILE_MARGIN,
			self.tile_side_len - (Self::TILE_MARGIN * 2.0),
			self.tile_side_len - (Self::TILE_MARGIN * 2.0),
			Color{r: 220.0, g: 220.0, b: 220.0, a: 1.0});
	}

	pub fn draw_bg(&self) {
        draw_rectangle(
			self.grid_rect.x,
			self.grid_rect.y,
			self.grid_rect.w,
			self.grid_rect.h,
			DARKGRAY);
		for x in 0..self.grid_size.0 {
			for y in 0..self.grid_size.1 {
				self.draw_empty_tile(&Coord{x: x, y: y});
			}
		}
	}

	pub fn get_tile_coord_from_pos(&self, pos: (f32, f32)) -> Option<Coord> {
		if !self.grid_rect.contains(Vec2::new(pos.0, pos.1)) {
			return None;
		}

		let x = ((pos.0 - self.grid_rect.x) / self.tile_side_len).floor() as usize;
		let y = ((pos.1 - self.grid_rect.y) / self.tile_side_len).floor() as usize;

		return Some(Coord{x: x, y: y})
	}
}

fn window_conf() -> Conf {
	// try workarounds from https://github.com/not-fl3/macroquad/issues/924
	Conf{
		window_title: "pourquoi".to_owned(),
		platform: Platform{
			webgl_version: WebGLVersion::WebGL2,
			..Default::default()
		},
		..Default::default()
	}
}

#[macroquad::main(window_conf)]
async fn main() {
	info!("logging check");

	let x_tiles = 48;
	let y_tiles = 32;
	//let mut g = Grid::<Tile>::new(x_tiles, y_tiles);

	let swidth = 1280;
	let sheight = 720;
	let grid_rt = render_target(swidth, sheight);
	//grid_rt.texture.set_filter(FilterMode::Linear);

	let mut grid_camera = Camera2D::from_display_rect(Rect::new(0.0, 0.0, swidth as f32, sheight as f32));
	grid_camera.render_target = Some(grid_rt);

    loop {
		set_camera(&grid_camera);

        clear_background(BLACK);
		let display = Display::new(swidth as f32, sheight as f32, x_tiles, y_tiles);
		//display.draw_bg();

        draw_line(40.0, 40.0, 100.0, 200.0, 15.0, BLUE);
        draw_circle(swidth as f32 - 30.0, sheight as f32 - 30.0, 15.0, YELLOW);
        draw_text("IT WORKS!", 20.0, 20.0, 30.0, DARKGRAY);


		let cam = Camera2D{
			offset: Vec2::new(0.1, 0.2),
			zoom: Vec2::ONE,
			target: Vec2::ZERO,
			..Default::default()
		};

		set_camera(&cam);
		//set_default_camera();
        //clear_background(BLACK);
		draw_texture_ex(
			&grid_camera.render_target.as_ref().unwrap().texture,
			0.0,
			0.0,
			WHITE,  // pourquoi?
			DrawTextureParams{
				dest_size: Some(Vec2::ONE),
				flip_y: true,
				..Default::default()
			},
		);

		if is_mouse_button_pressed(MouseButton::Left) {
			let mp = mouse_position();
			info!("mouse position {:?} {:?} {} {}",
					mp,
					cam,
					cam.world_to_screen(Vec2::new(mp.0, mp.1)),
					cam.screen_to_world(Vec2::new(mp.0, mp.1)));
			if let Some(grid_coord) = display.get_tile_coord_from_pos(mp) {
				info!("clicked on tile at {:?}", grid_coord);
			}
		}

        next_frame().await
    }
}
