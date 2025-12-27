// Any live cell with fewer than two live neighbours dies (referred to as underpopulation or exposure[2]).
// Any live cell with more than three live neighbours dies (referred to as overpopulation or overcrowding).
// Any live cell with two or three live neighbours lives, unchanged, to the next generation.
// Any dead cell with exactly three live neighbours will come to life.

use bevy::{
    prelude::*,
    time::common_conditions::on_timer,
    window::{PrimaryWindow, Window, WindowPlugin, WindowResolution},
};
use std::time::Duration;

const TILE_SIZE: f32 = 16.0;
const TILE_GAP: f32 = 2.0;
const ROWS: u32 = 20;
const COLS: u32 = 20;

const COLOR_BACKGROUND: Color = Color::srgb(0.8, 0.8, 0.8);
const COLOR_ALIVE: Color = Color::srgb(0.0, 0.0, 0.0);
const COLOR_DEAD: Color = Color::srgb(1.0, 1.0, 1.0);

#[derive(States, Debug, Clone, Eq, PartialEq, Hash, Default)]
enum GameState {
    #[default]
    Paused,
    Running,
}

// #[derive(Component)]
// struct StateLabel;

#[derive(Component)]
struct Cell;

#[derive(Resource)]
struct CurrentTile {
    row: usize,
    col: usize,
}

#[derive(Resource)]
struct Grid {
    cells: Vec<Option<Entity>>,
    state: Vec<bool>,
    new_state: Vec<bool>,
    rows: usize,
    cols: usize,
}

impl Grid {
    fn new(rows: usize, cols: usize) -> Self {
        let mut cells: Vec<Option<Entity>> = Vec::with_capacity(rows * cols);
        let mut state = Vec::with_capacity(rows * cols);
        let mut new_state = Vec::with_capacity(rows * cols);
        for _ in 0..(rows * cols) {
            cells.push(None);
            state.push(false);
            new_state.push(false);
        }
        Grid {
            cells,
            state,
            new_state,
            rows,
            cols,
        }
    }

    fn get(&self, row: usize, col: usize) -> Option<Entity> {
        if row < self.rows && col < self.cols {
            self.cells[row * self.cols + col]
        } else {
            None
        }
    }
}

fn main() {
    let window_width = grid_width() as f32 * 1.1;
    let window_height = grid_height() as f32 * 1.1;
    App::new()
        .insert_resource(ClearColor(COLOR_BACKGROUND))
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                resolution: WindowResolution::new(window_width as u32, window_height as u32),
                resizable: false,
                ..default()
            }),
            ..default()
        }))
        .init_state::<GameState>()
        .add_systems(Startup, (setup_camera, setup_grid, setup_ui))
        .add_systems(Update, pause_input)
        .add_systems(Update, mouse_button_input)
        .add_systems(
            Update,
            update_cells
                .run_if(in_state(GameState::Running))
                .run_if(on_timer(Duration::from_millis(500))),
        )
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
    for row in 0..ROWS {
        for column in 0..COLS {
            let tile_x = column as f32 * (TILE_SIZE + TILE_GAP) + (TILE_SIZE / 2.0);
            let tile_y = row as f32 * (TILE_SIZE + TILE_GAP) + (TILE_SIZE / 2.0);
            // create each cell
            let cell = commands
                .spawn((
                    Sprite {
                        color: COLOR_DEAD,
                        ..default()
                    },
                    Transform {
                        translation: Vec3::new(tile_x, tile_y, 0.0),
                        scale: Vec3::new(TILE_SIZE, TILE_SIZE, 1.0),
                        ..default()
                    },
                    Cell,
                ))
                .id();
            let index = (row * (grid.cols as u32) + column) as usize;
            grid.cells[index] = Some(cell);
        }
    }
    commands.insert_resource(grid);
}

fn setup_camera(mut commands: Commands) {
    let center_x = grid_width() as f32 / 2.0;
    let center_y = grid_height() as f32 / 2.0;
    commands.spawn((Camera2d, Transform::from_xyz(center_x, center_y, 0.)));
}

fn setup_ui(mut commands: Commands) {
    // TODO: ui and paused/running message

    let current_tile = CurrentTile { row: 0, col: 0 };
    commands.insert_resource(current_tile);
}

fn update_cells(mut grid: ResMut<Grid>, mut cell_query: Query<&mut Sprite, With<Cell>>) {
    let max_row = ROWS as i32 - 1;
    let max_col = COLS as i32 - 1;

    // check each cell and update grid.new_state to mark them alive or dead
    for row in 0..ROWS {
        for col in 0..COLS {
            let index = (row as usize) * grid.cols + (col as usize);
            let current_state = grid.state[index];

            let mut neighbors_alive = 0;
            // (row,col) offset of each adjacent cell
            let neighbors: [(i32, i32); 8] = [
                (-1, -1),
                (-1, 0),
                (-1, 1),
                (0, -1),
                (0, 1),
                (1, -1),
                (1, 0),
                (1, 1),
            ];
            for (y, x) in neighbors {
                let mut new_row = row as i32 + y;
                let mut new_col = col as i32 + x;

                // wrap around the edges
                if new_row < 0 {
                    new_row = max_row;
                } else if new_row > max_row {
                    new_row = 0;
                }
                if new_col < 0 {
                    new_col = max_col;
                } else if new_col > max_col {
                    new_col = 0;
                }

                let neighbor_index = (new_row as usize) * grid.cols + (new_col as usize);
                if grid.state[neighbor_index] {
                    neighbors_alive += 1;
                }
            }

            let mut new_state = current_state;
            if current_state == true {
                //Any live cell with fewer than two live neighbours dies
                if neighbors_alive < 2 {
                    new_state = false;
                // Any live cell with more than three live neighbours dies (referred to as overpopulation or overcrowding).
                } else if neighbors_alive > 3 {
                    new_state = false;
                }
            } else {
                // Any dead cell with exactly three live neighbours will come to life.
                if neighbors_alive == 3 {
                    new_state = true;
                }
            }

            grid.new_state[index] = new_state;
        }
    }

    // update the grid with the new state of each cell (alive or dead)
    for row in 0..ROWS {
        for col in 0..COLS {
            if let Some(cell) = grid.get(row as usize, col as usize) {
                if let Ok(mut sprite) = cell_query.get_mut(cell) {
                    let index = (row as usize) * grid.cols + (col as usize);
                    let current_state = grid.state[index];
                    let new_state = grid.new_state[index];
                    if current_state != new_state {
                        grid.state[index] = new_state;
                        if new_state {
                            sprite.color = COLOR_ALIVE;
                        } else {
                            sprite.color = COLOR_DEAD;
                        }
                    }
                }
            }
        }
    }
}

fn mouse_button_input(
    buttons: Res<ButtonInput<MouseButton>>,
    window: Single<&Window, With<PrimaryWindow>>,
    camera_query: Single<(&Camera, &GlobalTransform)>,
    mut grid: ResMut<Grid>,
    mut current_tile: ResMut<CurrentTile>,
    mut query: Query<&mut Sprite, With<Cell>>,
) {
    if buttons.pressed(MouseButton::Left) {
        let (camera, camera_transform) = camera_query.into_inner();
        if let Some(screen_position) = window.cursor_position() {
            if let Ok(ray) = camera.viewport_to_world(camera_transform, screen_position) {
                let world_position = ray.origin.truncate();
                let tile_x = (world_position.x / (TILE_SIZE + TILE_GAP)).floor();
                let tile_y = (world_position.y / (TILE_SIZE + TILE_GAP)).floor();
                if tile_x >= 0.0 && tile_y >= 0.0 {
                    let tile_x = tile_x as usize;
                    let tile_y = tile_y as usize;
                    // check that we're on a different tile
                    if !(tile_x == current_tile.col && tile_y == current_tile.row) {
                        if let Some(cell) = grid.get(tile_y, tile_x) {
                            if let Ok(mut sprite) = query.get_mut(cell) {
                                // toggle this cell alive or dead
                                let index = (tile_y as usize) * grid.cols + (tile_x as usize);
                                let new_state = !grid.state[index];
                                grid.state[index] = new_state;
                                sprite.color = if new_state { COLOR_ALIVE } else { COLOR_DEAD };
                            }
                            // store the current tile we're on
                            current_tile.col = tile_x;
                            current_tile.row = tile_y;
                        }
                    }
                }
            }
        }
    }
}

fn pause_input(
    keys: Res<ButtonInput<KeyCode>>,
    mut next_state: ResMut<NextState<GameState>>,
    state: Res<State<GameState>>,
) {
    if keys.just_pressed(KeyCode::Space) {
        match state.get() {
            GameState::Running => next_state.set(GameState::Paused),
            GameState::Paused => next_state.set(GameState::Running),
        }
    }
}
