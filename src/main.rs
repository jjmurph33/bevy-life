// Any live cell with fewer than two live neighbours dies (referred to as underpopulation or exposure).
// Any live cell with more than three live neighbours dies (referred to as overpopulation or overcrowding).
// Any live cell with two or three live neighbours lives, unchanged, to the next generation.
// Any dead cell with exactly three live neighbours will come to life.

use bevy::{
    dev_tools::fps_overlay::{FpsOverlayConfig, FpsOverlayPlugin, FrameTimeGraphConfig},
    input_focus::InputFocus,
    prelude::*,
    time::common_conditions::on_timer,
    window::{PrimaryWindow, Window, WindowPlugin, WindowResolution},
};
use rand::{Rng, SeedableRng};
use std::time::Duration;

const TILE_SIZE: f32 = 16.0;
const TILE_GAP: f32 = 2.0;
const ROWS: usize = 30;
const COLS: usize = 30;

const COLOR_BACKGROUND: Color = Color::srgb(0.8, 0.8, 0.8);
const COLOR_ALIVE: Color = Color::srgb(0.0, 0.0, 0.0);
const COLOR_DEAD: Color = Color::srgb(1.0, 1.0, 1.0);
const COLOR_BUTTON: Color = Color::srgb(0.15, 0.15, 0.15);
const COLOR_BUTTON_HOVERED: Color = Color::srgb(0.25, 0.25, 0.25);
const COLOR_DEBUG_TEXT: Color = Color::srgb(1.0, 1.0, 1.0);

#[derive(States, Debug, Clone, Eq, PartialEq, Hash, Default)]
enum GameState {
    #[default]
    Starting,
    Paused,
    Running,
}

#[derive(Resource)]
struct GameRng(rand::rngs::SmallRng);

#[derive(Component)]
struct StateButton;

#[derive(Component)]
struct RandomButton;

#[derive(Component)]
struct ClearButton;

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
    state: [[bool; COLS]; ROWS],
    new_state: [[bool; COLS]; ROWS],
    rows: usize,
    cols: usize,
}

impl Grid {
    fn new(rows: usize, cols: usize) -> Self {
        let mut cells: Vec<Option<Entity>> = Vec::with_capacity(rows * cols);
        for _ in 0..(rows * cols) {
            cells.push(None);
        }
        Grid {
            cells,
            state: [[false; COLS]; ROWS],
            new_state: [[false; COLS]; ROWS],
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
    let window_height = grid_height() as f32 * 1.1 + 50.0;
    App::new()
        .insert_resource(ClearColor(COLOR_BACKGROUND))
        .add_plugins((
            DefaultPlugins.set(WindowPlugin {
                primary_window: Some(Window {
                    resolution: WindowResolution::new(window_width as u32, window_height as u32),
                    resizable: false,
                    title: "Life".to_string(),
                    ..default()
                }),
                ..default()
            }),
            FpsOverlayPlugin {
                config: FpsOverlayConfig {
                    text_config: TextFont {
                        font_size: 20.0,
                        ..default()
                    },
                    text_color: COLOR_DEBUG_TEXT,
                    enabled: false,
                    frame_time_graph_config: FrameTimeGraphConfig {
                        enabled: false,
                        ..default()
                    },
                    ..default()
                },
            },
        ))
        .init_state::<GameState>()
        .init_resource::<InputFocus>()
        .add_systems(Startup, (setup_camera, setup_grid, setup_ui))
        .add_systems(
            Update,
            (
                pause_input,
                mouse_button_input,
                state_button_update,
                random_button_update,
                clear_button_update,
                debug_text_update,
            ),
        )
        .add_systems(
            Update,
            update_cells
                .run_if(in_state(GameState::Running))
                .run_if(on_timer(Duration::from_millis(500))),
        )
        .run();
}

fn grid_width() -> u32 {
    (TILE_SIZE + TILE_GAP) as u32 * COLS as u32
}

fn grid_height() -> u32 {
    (TILE_SIZE + TILE_GAP) as u32 * ROWS as u32
}

fn setup_grid(mut commands: Commands, mut next_state: ResMut<NextState<GameState>>) {
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
            let index = (row * grid.cols + column) as usize;
            grid.cells[index] = Some(cell);
        }
    }
    commands.insert_resource(grid);

    // the last tile that was selected
    commands.insert_resource(CurrentTile { row: 0, col: 0 });

    next_state.set(GameState::Paused);
}

fn setup_camera(mut commands: Commands) {
    let center_x = grid_width() as f32 / 2.0;
    let center_y = grid_height() as f32 / 2.0;
    commands.spawn((Camera2d, Transform::from_xyz(center_x, center_y, 0.)));
}

fn setup_ui(mut commands: Commands) {
    // the state label
    // commands.spawn((
    //     Text::new("Paused"),
    //     Node {
    //         position_type: PositionType::Absolute,
    //         bottom: px(5),
    //         left: px(20),
    //         ..default()
    //     },
    //     TextColor(COLOR_TEXT),
    //     StateLabel,
    // ));

    // initialize the RNG
    commands.insert_resource(GameRng(rand::rngs::SmallRng::from_os_rng()));

    // create the buttons
    commands.spawn((
        Node {
            position_type: PositionType::Absolute,
            bottom: px(5),
            left: px(20),
            margin: UiRect::all(Val::Px(5.0)),
            column_gap: Val::Px(10.0),
            ..default()
        },
        children![
            (
                Button,
                StateButton,
                Node {
                    width: px(100),
                    height: px(25),
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    ..default()
                },
                BorderColor::all(Color::WHITE),
                BorderRadius::MAX,
                children![(
                    Text::new("Run"),
                    TextColor(Color::srgb(0.9, 0.9, 0.9)),
                    TextShadow::default(),
                )]
            ),
            (
                Button,
                RandomButton,
                Node {
                    width: px(100),
                    height: px(25),
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    ..default()
                },
                BorderColor::all(Color::WHITE),
                BorderRadius::MAX,
                children![(
                    Text::new("Random"),
                    TextColor(Color::srgb(0.9, 0.9, 0.9)),
                    TextShadow::default(),
                )]
            ),
            (
                Button,
                ClearButton,
                Node {
                    width: px(100),
                    height: px(25),
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    ..default()
                },
                BorderColor::all(Color::WHITE),
                BorderRadius::MAX,
                children![(
                    Text::new("Clear"),
                    TextColor(Color::srgb(0.9, 0.9, 0.9)),
                    TextShadow::default(),
                )]
            )
        ],
    ));
}

fn update_cells(mut grid: ResMut<Grid>, mut cell_query: Query<&mut Sprite, With<Cell>>) {
    for row in 0..ROWS {
        for col in 0..COLS {
            let current_state = grid.state[row][col];
            let mut new_state = current_state;
            let num_neighbors_alive = living_neighbors(&grid.state, row, col);
            if current_state == true {
                //Any live cell with fewer than two live neighbours dies
                if num_neighbors_alive < 2 {
                    new_state = false;
                // Any live cell with more than three live neighbours dies
                } else if num_neighbors_alive > 3 {
                    new_state = false;
                }
            } else {
                // Any dead cell with exactly three live neighbours will come to life.
                if num_neighbors_alive == 3 {
                    new_state = true;
                }
            }
            grid.new_state[row][col] = new_state;
        }
    }

    // update the grid with the new state of each cell (alive or dead)
    for row in 0..ROWS {
        for col in 0..COLS {
            let current_state = grid.state[row][col];
            let new_state = grid.new_state[row][col];
            if current_state != new_state {
                if let Some(cell) = grid.get(row as usize, col as usize) {
                    if let Ok(mut sprite) = cell_query.get_mut(cell) {
                        grid.state[row][col] = new_state;
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

fn living_neighbors(state: &[[bool; COLS]; ROWS], row: usize, col: usize) -> u32 {
    let mut neighbors_alive = 0;
    let max_row = ROWS as i32 - 1;
    let max_col = COLS as i32 - 1;
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

        if state[new_row as usize][new_col as usize] {
            neighbors_alive += 1;
        }
    }
    neighbors_alive
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
                    let col = tile_x as usize;
                    let row = tile_y as usize;
                    // check that we're on a different tile
                    if !(col == current_tile.col && row == current_tile.row) {
                        if let Some(cell) = grid.get(row, col) {
                            if let Ok(mut sprite) = query.get_mut(cell) {
                                // toggle this cell alive or dead
                                let new_state = !grid.state[row][col];
                                grid.state[row][col] = new_state;
                                sprite.color = if new_state { COLOR_ALIVE } else { COLOR_DEAD };
                            }
                            // store the current tile we're on
                            current_tile.col = col;
                            current_tile.row = row;
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
    button_q: Single<&Children, With<StateButton>>,
    mut button_text_q: Query<&mut Text>,
) {
    if keys.just_pressed(KeyCode::Space) {
        let state_button_children = button_q.into_inner();
        let mut text = button_text_q.get_mut(state_button_children[0]).unwrap();
        match state.get() {
            GameState::Starting => (),
            GameState::Running => {
                next_state.set(GameState::Paused);
                **text = "Run".to_string();
            }
            GameState::Paused => {
                next_state.set(GameState::Running);
                **text = "Pause".to_string();
            }
        }
    }
}

fn state_button_update(
    interaction_query: Single<
        (
            &Interaction,
            &mut BackgroundColor,
            &mut BorderColor,
            &mut Button,
            &Children,
        ),
        (Changed<Interaction>, With<StateButton>),
    >,
    mut text_query: Query<&mut Text>,
    state: Res<State<GameState>>,
    mut next_state: ResMut<NextState<GameState>>,
) {
    let (interaction, mut color, mut border_color, mut button, children) =
        interaction_query.into_inner();
    let state = state.get();
    let mut text = text_query.get_mut(children[0]).unwrap();
    match *interaction {
        Interaction::Pressed => {
            **text = match state {
                GameState::Starting => "Loading...".to_string(),
                GameState::Paused => {
                    next_state.set(GameState::Running);
                    "Pause".to_string()
                }
                GameState::Running => {
                    next_state.set(GameState::Paused);
                    "Run".to_string()
                }
            };
        }
        Interaction::Hovered => {
            *color = COLOR_BUTTON_HOVERED.into();
            *border_color = BorderColor::all(Color::WHITE);
        }
        Interaction::None => {
            **text = match state {
                GameState::Starting => "Loading...".to_string(),
                GameState::Paused => "Run".to_string(),
                GameState::Running => "Pause".to_string(),
            };
            *color = COLOR_BUTTON.into();
            *border_color = BorderColor::all(Color::BLACK);
        }
    }
    button.set_changed();
}

fn random_button_update(
    interaction_query: Single<
        (
            &Interaction,
            &mut BackgroundColor,
            &mut BorderColor,
            &mut Button,
        ),
        (Changed<Interaction>, With<RandomButton>),
    >,
    mut grid: ResMut<Grid>,
    mut rng: ResMut<GameRng>,
    mut cell_q: Query<&mut Sprite, With<Cell>>,
) {
    let (interaction, mut color, mut border_color, mut button) = interaction_query.into_inner();
    match *interaction {
        Interaction::Pressed => {
            for row in 0..ROWS {
                for col in 0..COLS {
                    // 33% chance of true
                    let value = rng.0.random_bool(0.33);
                    grid.state[row][col] = value;
                    grid.new_state[row][col] = value;
                    if let Some(cell) = grid.get(row as usize, col as usize) {
                        if let Ok(mut sprite) = cell_q.get_mut(cell) {
                            grid.state[row][col] = value;
                            if value {
                                sprite.color = COLOR_ALIVE;
                            } else {
                                sprite.color = COLOR_DEAD;
                            }
                        }
                    }
                }
            }
        }
        Interaction::Hovered => {
            *color = COLOR_BUTTON_HOVERED.into();
            *border_color = BorderColor::all(Color::WHITE);
        }
        Interaction::None => {
            *color = COLOR_BUTTON.into();
            *border_color = BorderColor::all(Color::BLACK);
        }
    }
    button.set_changed();
}

fn clear_button_update(
    interaction_query: Single<
        (
            &Interaction,
            &mut BackgroundColor,
            &mut BorderColor,
            &mut Button,
        ),
        (Changed<Interaction>, With<ClearButton>),
    >,
    mut grid: ResMut<Grid>,
    mut cell_q: Query<&mut Sprite, With<Cell>>,
) {
    let (interaction, mut color, mut border_color, mut button) = interaction_query.into_inner();
    match *interaction {
        Interaction::Pressed => {
            for row in 0..ROWS {
                for col in 0..COLS {
                    let value = false;
                    grid.state[row][col] = value;
                    grid.new_state[row][col] = value;
                    if let Some(cell) = grid.get(row as usize, col as usize) {
                        if let Ok(mut sprite) = cell_q.get_mut(cell) {
                            grid.state[row][col] = value;
                            sprite.color = COLOR_DEAD;
                        }
                    }
                }
            }
        }
        Interaction::Hovered => {
            *color = COLOR_BUTTON_HOVERED.into();
            *border_color = BorderColor::all(Color::WHITE);
        }
        Interaction::None => {
            *color = COLOR_BUTTON.into();
            *border_color = BorderColor::all(Color::BLACK);
        }
    }
    button.set_changed();
}

fn debug_text_update(input: Res<ButtonInput<KeyCode>>, mut overlay: ResMut<FpsOverlayConfig>) {
    if input.just_pressed(KeyCode::F11) {
        overlay.enabled = !overlay.enabled;
    }
}
