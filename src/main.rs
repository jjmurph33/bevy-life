// Any live cell with fewer than two live neighbours dies (referred to as underpopulation or exposure[2]).
// Any live cell with more than three live neighbours dies (referred to as overpopulation or overcrowding).
// Any live cell with two or three live neighbours lives, unchanged, to the next generation.
// Any dead cell with exactly three live neighbours will come to life.

use bevy::{
    prelude::*,
    window::{PrimaryWindow, Window, WindowPlugin, WindowResolution},
};

const TILE_SIZE: f32 = 32.0;
const TILE_GAP: f32 = 1.0;
const ROWS: u32 = 10;
const COLS: u32 = 10;

const BACKGROUND_COLOR: Color = Color::srgb(0.9, 0.9, 0.9);
const CELL_COLOR: Color = Color::srgb(0.1, 0.1, 0.1);

#[derive(Component)]
struct Alive(bool);

#[derive(Component)]
struct Cell;

#[derive(Resource)]
struct Grid {
    cells: Vec<Entity>,
    rows: usize,
    cols: usize,
}

impl Grid {
    fn new(rows: usize, cols: usize) -> Self {
        Grid {
            cells: Vec::with_capacity(rows * cols),
            rows,
            cols,
        }
    }

    fn get(&self, row: usize, col: usize) -> Option<Entity> {
        if row < self.rows && col < self.cols {
            Some(self.cells[row * self.cols + col])
        } else {
            None
        }
    }

    fn set(&mut self, row: usize, col: usize, entity: Entity) {
        if row < self.rows && col < self.cols {
            let index = row * self.cols + col;
            if index < self.cells.len() {
                self.cells[index] = entity;
            } else {
                self.cells.push(entity);
            }
        }
    }
}

fn main() {
    App::new()
        .insert_resource(ClearColor(BACKGROUND_COLOR))
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                resolution: WindowResolution::new(grid_width() * 2, grid_height() * 2),
                resizable: false,
                ..default()
            }),
            ..default()
        }))
        .add_systems(Startup, (setup_camera, setup_grid))
        .add_systems(Update, (mouse_button_input, update_cells))
        .run();
}

fn grid_width() -> u32 {
    (TILE_SIZE + TILE_GAP) as u32 * COLS
}

fn grid_height() -> u32 {
    (TILE_SIZE + TILE_GAP) as u32 * ROWS
}

fn setup_grid(mut commands: Commands) {
    let mut grid = Grid::new(ROWS as usize, COLS as usize);

    let mut first = true;

    for row in 0..ROWS {
        for column in 0..COLS {
            let tile_x = column as f32 * (TILE_SIZE + TILE_GAP) + (TILE_SIZE / 2.0);
            let tile_y = row as f32 * (TILE_SIZE + TILE_GAP) + (TILE_SIZE / 2.0);

            let cell = commands
                .spawn((
                    Sprite {
                        color: CELL_COLOR,
                        ..default()
                    },
                    Transform {
                        //translation: tile_position.extend(0.0),
                        translation: Vec3::new(tile_x, tile_y, 0.0),
                        scale: Vec3::new(TILE_SIZE, TILE_SIZE, 1.0),
                        ..default()
                    },
                    Cell,
                    Alive(first),
                ))
                .id();

            println!(
                "Spawing cell at row {} col {} (position {} {})",
                row, column, tile_x, tile_y
            );

            grid.set(row as usize, column as usize, cell);

            first = false;
        }
    }

    commands.insert_resource(grid);
}

fn setup_camera(mut commands: Commands) {
    let center_x = grid_width() as f32 / 2.0;
    let center_y = grid_height() as f32 / 2.0;
    commands.spawn((Camera2d, Transform::from_xyz(center_x, center_y, 0.)));
}

fn update_cells(grid: Res<Grid>, cell_query: Query<&mut Alive, With<Cell>>) {
    print!("Alive: ");
    for row in 0..ROWS {
        for column in 0..COLS {
            if let Some(cell) = grid.get(row as usize, column as usize) {
                if let Ok(alive) = cell_query.get(cell) {
                    if alive.0 {
                        print!("{} ", cell)
                    }
                }
            };
        }
    }
    print!("\n");
}

fn mouse_button_input(
    buttons: Res<ButtonInput<MouseButton>>,
    window: Single<&Window, With<PrimaryWindow>>,
    camera_query: Single<(&Camera, &GlobalTransform)>,
    grid: Res<Grid>,
    mut query: Query<(&mut Sprite, &mut Alive), With<Cell>>,
) {
    if buttons.just_pressed(MouseButton::Left) {
        let (camera, camera_transform) = camera_query.into_inner();

        if let Some(screen_position) = window.cursor_position() {
            //println!("Screen coords: {}/{}", screen_position.x, screen_position.y);
            if let Ok(ray) = camera.viewport_to_world(camera_transform, screen_position) {
                let world_position = ray.origin.truncate();
                //println!("World coords: {}/{}", world_position.x, world_position.y);

                let tile_x = (world_position.x / (TILE_SIZE + TILE_GAP)).floor();
                let tile_y = (world_position.y / (TILE_SIZE + TILE_GAP)).floor();
                //println!("Tiles: {}/{}", tile_x, tile_y);

                if tile_x >= 0.0 && tile_y >= 0.0 {
                    let tile_x = tile_x as usize;
                    let tile_y = tile_y as usize;

                    if let Some(cell) = grid.get(tile_y, tile_x) {
                        if let Ok((mut sprite, mut alive)) = query.get_mut(cell) {
                            alive.0 = !alive.0;
                            sprite.color = if alive.0 {
                                Color::srgb(0.2, 0.8, 0.2) // Green when alive
                            } else {
                                CELL_COLOR // Dark when dead
                            };
                            println!(
                                "Clicked tile (row {tile_y}, col {tile_x}) - Alive: {}",
                                alive.0
                            );
                        }
                    }
                }
            }
        }
    }
}
