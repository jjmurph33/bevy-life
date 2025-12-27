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

const TILE_SIZE: f32 = 32.0;
const TILE_GAP: f32 = 1.0;
const ROWS: u32 = 10;
const COLS: u32 = 10;

const COLOR_BACKGROUND: Color = Color::srgb(0.9, 0.9, 0.9);
const COLOR_ALIVE: Color = Color::srgb(0.2, 0.8, 0.2);
const COLOR_DEAD: Color = Color::srgb(0.1, 0.1, 0.1);

#[derive(States, Debug, Clone, Eq, PartialEq, Hash, Default)]
enum GameState {
    #[default]
    Running,
    Paused,
}

#[derive(Component)]
struct StateLabel;

#[derive(Component)]
struct Alive(bool);

#[derive(Component)]
struct Cell;

#[derive(Resource)]
struct Grid {
    cells: Vec<Entity>,
    new_state: Vec<bool>,
    rows: usize,
    cols: usize,
}

impl Grid {
    fn new(rows: usize, cols: usize) -> Self {
        Grid {
            cells: Vec::with_capacity(rows * cols),
            new_state: Vec::with_capacity(rows * cols),
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
                self.new_state[index] = false;
            } else {
                self.cells.push(entity);
                self.new_state.push(false);
            }
        }
    }
}

fn main() {
    App::new()
        .insert_resource(ClearColor(COLOR_BACKGROUND))
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                resolution: WindowResolution::new(grid_width() * 2, grid_height() * 2),
                resizable: false,
                ..default()
            }),
            ..default()
        }))
        .init_state::<GameState>()
        .add_systems(Startup, (setup_camera, setup_grid, setup_display))
        .add_systems(Update, pause_input)
        .add_systems(Update, mouse_button_input)
        .add_systems(Update, update_cells.run_if(in_state(GameState::Running)).run_if(on_timer(Duration::from_secs(1))))
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
                    Alive(false),
                ))
                .id();
            grid.set(row as usize, column as usize, cell);
        }
    }
    commands.insert_resource(grid);
}

fn setup_camera(mut commands: Commands) {
    let center_x = grid_width() as f32 / 2.0;
    let center_y = grid_height() as f32 / 2.0;
    commands.spawn((Camera2d, Transform::from_xyz(center_x, center_y, 0.)));
}

fn setup_display(mut commands: Commands) {

}


fn update_cells(mut grid: ResMut<Grid>, mut cell_query: Query<(&mut Sprite,&mut Alive), With<Cell>>) {
    // check each cell and update grid.new_state to mark them alive or dead
    for row in 0..ROWS {
        for col in 0..COLS {
            if let Some(cell) = grid.get(row as usize, col as usize) {
                if let Ok((_,alive)) = cell_query.get_mut(cell) {
                    let current_state = alive.0;
                    let mut neighbors_alive = 0;
                    // (row,col) offset of each adjacent cell
                    let neighbors: [(i32,i32);8] = [(-1,-1),(-1,0),(-1,1),(0,-1),(0,1),(1,-1),(1,0),(1,1)];
                    for (y,x) in neighbors {
                        let new_row = (row as i32 + y) as usize;
                        let new_col = (col as i32 + x) as usize;
                        if let Some(neighbor_cell) = grid.get(new_row,new_col) {
                            if let Ok((_,alive)) = cell_query.get_mut(neighbor_cell) {
                                if alive.0 {
                                    neighbors_alive += 1;
                                }
                            }
                        }
                    }
                    let mut new_state = current_state;
                    //println!("current state: {} living neighbors: {}",current_state,neighbors_alive);
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
                    let index = (row as usize) * grid.cols + (col as usize);
                    grid.new_state[index] = new_state;
                }
            };
        }
    }

    // update the grid with the new state of each cell (alive or dead)
    for row in 0..ROWS {
        for col in 0..COLS {
            if let Some(cell) = grid.get(row as usize, col as usize) {
                if let Ok((mut sprite,mut alive)) = cell_query.get_mut(cell) {
                    let current_state = alive.0;
                    let index = (row as usize) * grid.cols + (col as usize);
                    let new_state = grid.new_state[index];
                    if current_state != new_state {
                        //println!("new state: {}",new_state);
                        alive.0 = new_state;
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
                                COLOR_ALIVE
                            } else {
                                COLOR_DEAD
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

fn pause_input(
    keys: Res<ButtonInput<KeyCode>>,
    mut next_state: ResMut<NextState<GameState>>,
    state: Res<State<GameState>>,
) {
    if keys.just_pressed(KeyCode::Space) {
        match state.get() {
            GameState::Running => {
                println!("pausing");
                next_state.set(GameState::Paused)
            },
            GameState::Paused => {
                println!("running");
                next_state.set(GameState::Running)
            },
        }
    }
}
