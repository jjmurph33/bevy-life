// Any live cell with fewer than two live neighbours dies (referred to as underpopulation or exposure).
// Any live cell with more than three live neighbours dies (referred to as overpopulation or overcrowding).
// Any live cell with two or three live neighbours lives, unchanged, to the next generation.
// Any dead cell with exactly three live neighbours will come to life.

use bevy::{
    input_focus::InputFocus,
    log::{Level, LogPlugin},
    prelude::*,
    tasks::ComputeTaskPool,
    window::{PrimaryWindow, Window, WindowPlugin, WindowResolution},
};

#[cfg(not(target_arch = "wasm32"))]
use bevy::dev_tools::fps_overlay::{FpsOverlayConfig, FpsOverlayPlugin, FrameTimeGraphConfig};

use rand::{Rng, SeedableRng};
use std::sync::{Arc, Mutex};
use std::time::Duration;
//use std::time::Instant;

const TILE_SIZE: f32 = 8.0;
const TILE_GAP: f32 = 1.0;
const ROWS: usize = 50;
const COLS: usize = 100;
const CELL_LIFETIME: u8 = 6; // number of ticks for a dead cell to decay

const COLOR_BACKGROUND: Color = Color::srgb(0.8, 0.8, 0.8);
const COLOR_ALIVE: Color = Color::srgb(0.0, 0.0, 0.0);
const COLOR_DEAD: Color = Color::srgb(1.0, 1.0, 1.0);
const COLOR_DECAY1: Color = Color::srgb(0.9, 0.9, 0.9);
const COLOR_DECAY2: Color = Color::srgb(0.8, 0.8, 0.8);
const COLOR_DECAY3: Color = Color::srgb(0.7, 0.7, 0.7);
const COLOR_DECAY4: Color = Color::srgb(0.6, 0.6, 0.6);
const COLOR_DECAY5: Color = Color::srgb(0.5, 0.5, 0.5);
const COLOR_BUTTON: Color = Color::srgb(0.15, 0.15, 0.15);
const COLOR_BUTTON_HOVERED: Color = Color::srgb(0.25, 0.25, 0.25);
const COLOR_BUTTON_TEXT: Color = Color::srgb(0.9, 0.9, 0.9);
const COLOR_DEBUG_TEXT: Color = Color::srgb(0.0, 0.5, 0.0);
const COLOR_TEXT: Color = Color::srgb(0.0, 0.0, 0.0);

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
struct DebugText;

#[derive(Component)]
struct StatusText;

#[derive(Component)]
struct Cell;

#[derive(Resource, Debug)]
struct CurrentTile {
    row: usize,
    col: usize,
    //timer: f32,
}

#[derive(Resource)]
struct TickDelay(u64);
impl Default for TickDelay {
    fn default() -> Self {
        TickDelay(500)
    }
}

#[derive(Resource)]
struct Grid {
    cells: Vec<Option<Entity>>,
    state: [[u8; COLS]; ROWS], // CELL_LIFETIME = alive, less than CELL_LIFETIME = dead (decaying)
    result_rows: Arc<Vec<Mutex<[bool; COLS]>>>, // mutex-protected buffer for each row
    num_alive: usize,
}

impl Grid {
    fn new(rows: usize, cols: usize) -> Self {
        let mut cells: Vec<Option<Entity>> = Vec::with_capacity(rows * cols);
        for _ in 0..(rows * cols) {
            cells.push(None);
        }
        let result_rows: Arc<Vec<Mutex<[bool; COLS]>>> =
            Arc::new((0..ROWS).map(|_| Mutex::new([false; COLS])).collect());
        Grid {
            cells,
            state: [[0; COLS]; ROWS],
            result_rows,
            num_alive: 0,
        }
    }
    fn get(&self, row: usize, col: usize) -> Option<Entity> {
        if row < ROWS && col < COLS {
            self.cells[row * COLS + col]
        } else {
            None
        }
    }
}

fn main() {
    let window_width = grid_width() as f32 * 1.01;
    let window_height = grid_height() as f32 * 1.01 + 50.0;
    let mut app = App::new();
    app.insert_resource(ClearColor(COLOR_BACKGROUND))
        .add_plugins(
            DefaultPlugins
                .set(WindowPlugin {
                    primary_window: Some(Window {
                        resolution: WindowResolution::new(
                            window_width as u32,
                            window_height as u32,
                        ),
                        resizable: false,
                        title: "Life".to_string(),
                        ..default()
                    }),
                    ..default()
                })
                .set(LogPlugin {
                    level: Level::ERROR,
                    ..default()
                }),
        );

    #[cfg(not(target_arch = "wasm32"))]
    app.add_plugins(FpsOverlayPlugin {
        config: FpsOverlayConfig {
            text_config: TextFont {
                font_size: FontSize::Px(20.0),
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
    });

    app.init_state::<GameState>()
        .init_resource::<InputFocus>()
        .init_resource::<TickDelay>()
        .add_systems(Startup, (setup_camera, setup_grid, setup_ui))
        .add_systems(
            Update,
            (
                keyboard_input,
                mouse_button_input,
                state_button_update,
                random_button_update,
                clear_button_update,
                #[cfg(not(target_arch = "wasm32"))]
                debug_text_update,
                status_text_update,
            ),
        )
        .add_systems(
            Update,
            update_cells
                .run_if(in_state(GameState::Running))
                // update the grid when the tick delay timer finishes
                .run_if(
                    |mut timer: Local<Timer>, time: Res<Time>, tick_delay: Res<TickDelay>| {
                        let delay = Duration::from_millis(tick_delay.0);
                        if timer.duration() != delay {
                            // update the timer if the delay has changed
                            *timer = Timer::new(delay, TimerMode::Repeating);
                        }
                        timer.tick(time.delta());
                        timer.just_finished()
                    },
                ),
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

    // initialize the grid with a horizontal line in the middle
    let active_row = ROWS / 2;
    let active_col_start = COLS / 2 - (COLS / 4);
    let active_col_end = COLS / 2 + (COLS / 4);

    for row in 0..ROWS {
        for column in 0..COLS {
            // set the starting alive cells
            let alive =
                if row == active_row && column >= active_col_start && column < active_col_end {
                    grid.state[row][column] = CELL_LIFETIME;
                    true
                } else {
                    false
                };

            let tile_x = column as f32 * (TILE_SIZE + TILE_GAP) + (TILE_SIZE / 2.0);
            let tile_y = row as f32 * (TILE_SIZE + TILE_GAP) + (TILE_SIZE / 2.0);

            // create each cell
            let cell = commands
                .spawn((
                    Sprite {
                        color: if alive { COLOR_ALIVE } else { COLOR_DEAD },
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
            let index = (row * COLS + column) as usize;
            grid.cells[index] = Some(cell);
        }
    }

    commands.insert_resource(grid);

    // the last tile that was selected
    commands.insert_resource(CurrentTile {
        row: 0,
        col: 0,
        //timer: 0.0,
    });

    next_state.set(GameState::Running);
}

fn setup_camera(mut commands: Commands) {
    let center_x = grid_width() as f32 / 2.0;
    let center_y = grid_height() as f32 / 2.0 - 25.0;
    commands.spawn((Camera2d, Transform::from_xyz(center_x, center_y, 0.)));
}

fn setup_ui(mut commands: Commands) {
    // initialize the RNG
    commands.insert_resource(GameRng(rand::rngs::SmallRng::from_os_rng()));

    // create the buttons
    commands.spawn((
        Node {
            position_type: PositionType::Absolute,
            bottom: px(5),
            left: px(10),
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
                    border_radius: BorderRadius::MAX,
                    ..default()
                },
                BorderColor::all(Color::WHITE),
                children![(
                    Text::new("Run"),
                    TextColor(COLOR_BUTTON_TEXT),
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
                    border_radius: BorderRadius::MAX,
                    ..default()
                },
                BorderColor::all(Color::WHITE),
                children![(
                    Text::new("Random"),
                    TextColor(COLOR_BUTTON_TEXT),
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
                    border_radius: BorderRadius::MAX,
                    ..default()
                },
                BorderColor::all(Color::WHITE),
                children![(
                    Text::new("Clear"),
                    TextColor(COLOR_BUTTON_TEXT),
                    TextShadow::default(),
                )]
            )
        ],
    ));

    // debug text
    commands.spawn((
        Text::new(""),
        Node {
            position_type: PositionType::Absolute,
            top: px(0),
            right: px(30),
            ..default()
        },
        TextColor(COLOR_DEBUG_TEXT),
        Visibility::Hidden,
        DebugText,
    ));

    // status text
    commands.spawn((
        Text::new(""),
        TextFont {
            font_size: FontSize::Px(16.0),
            ..default()
        },
        Node {
            position_type: PositionType::Absolute,
            bottom: px(5),
            left: px(350),
            ..default()
        },
        TextColor(COLOR_TEXT),
        StatusText,
    ));
}

fn update_cells(
    mut grid: ResMut<Grid>,
    mut cell_query: Query<&mut Sprite, With<Cell>>,
    //debug_text: Single<&mut Text, With<DebugText>>,
) {
    //let timer = Instant::now();
    let state = &grid.state.clone();

    // spawn a task for each row
    ComputeTaskPool::get().scope(|s| {
        for row in 0..ROWS {
            let result_rows = grid.result_rows.clone();
            s.spawn(async move {
                let mut section = result_rows[row].lock().unwrap();
                update_row(&mut *section, &state, row);
            });
        }
    });

    // collect the results from each row
    let mut new_state = [[false; COLS]; ROWS];
    for row in 0..ROWS {
        let section = grid.result_rows[row].lock().unwrap();
        new_state[row] = *section;
    }

    let mut num_alive = 0;

    // update the grid with the new state of each cell (alive or dead)
    for row in 0..ROWS {
        for col in 0..COLS {
            let alive = new_state[row][col];
            if let Some(cell) = grid.get(row as usize, col as usize) {
                if let Ok(mut sprite) = cell_query.get_mut(cell) {
                    if alive {
                        grid.state[row][col] = CELL_LIFETIME;
                        num_alive += 1;
                    } else {
                        // dead cells decay each tick
                        if grid.state[row][col] > 0 {
                            grid.state[row][col] -= 1;
                        }
                    }
                    match grid.state[row][col] {
                        6 => sprite.color = COLOR_ALIVE,
                        5 => sprite.color = COLOR_DECAY5,
                        4 => sprite.color = COLOR_DECAY4,
                        3 => sprite.color = COLOR_DECAY3,
                        2 => sprite.color = COLOR_DECAY2,
                        1 => sprite.color = COLOR_DECAY1,
                        _ => sprite.color = COLOR_DEAD,
                    }
                }
            }
        }
    }

    grid.num_alive = num_alive;

    //let elapsed = timer.elapsed().as_nanos();
    //let mut debug_text = debug_text.into_inner();
    //**debug_text = format!("Update time: {} ns", elapsed);
}

fn update_row(section: &mut [bool; COLS], state: &[[u8; COLS]; ROWS], row: usize) {
    for col in 0..COLS {
        let alive = if state[row][col] == CELL_LIFETIME {
            true
        } else {
            false
        };
        let mut new_state = alive;
        let num_neighbors_alive = living_neighbors(state, row, col);
        if alive {
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
        section[col] = new_state;
    }
}

fn living_neighbors(state: &[[u8; COLS]; ROWS], row: usize, col: usize) -> u32 {
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

        if state[new_row as usize][new_col as usize] == CELL_LIFETIME {
            neighbors_alive += 1;
        }
    }
    neighbors_alive
}

// synchronous version of update_cells (may be needed for WASM)
#[allow(dead_code)]
fn update_cells_sync(
    mut grid: ResMut<Grid>,
    mut cell_query: Query<&mut Sprite, With<Cell>>,
    //debug_text: Single<&mut Text, With<DebugText>>,
) {
    //let timer = Instant::now();
    let state = &grid.state.clone();
    let mut new_state = [[false; COLS]; ROWS];

    for row in 0..ROWS {
        for col in 0..COLS {
            let alive = if state[row][col] == CELL_LIFETIME {
                true
            } else {
                false
            };
            let mut new_cell_state = alive;
            let num_neighbors_alive = living_neighbors(state, row, col);
            if alive {
                //Any live cell with fewer than two live neighbours dies
                if num_neighbors_alive < 2 {
                    new_cell_state = false;
                // Any live cell with more than three live neighbours dies
                } else if num_neighbors_alive > 3 {
                    new_cell_state = false;
                }
            } else {
                // Any dead cell with exactly three live neighbours will come to life.
                if num_neighbors_alive == 3 {
                    new_cell_state = true;
                }
            }
            new_state[row][col] = new_cell_state;
        }
    }

    let mut num_alive = 0;

    // update the grid with the new state of each cell (alive or dead)
    for row in 0..ROWS {
        for col in 0..COLS {
            let alive = new_state[row][col];
            if let Some(cell) = grid.get(row as usize, col as usize) {
                if let Ok(mut sprite) = cell_query.get_mut(cell) {
                    if alive {
                        grid.state[row][col] = CELL_LIFETIME;
                        num_alive += 1;
                    } else {
                        // dead cells decay each tick
                        if grid.state[row][col] > 0 {
                            grid.state[row][col] -= 1;
                        }
                    }
                    match grid.state[row][col] {
                        6 => sprite.color = COLOR_ALIVE,
                        5 => sprite.color = COLOR_DECAY5,
                        4 => sprite.color = COLOR_DECAY4,
                        3 => sprite.color = COLOR_DECAY3,
                        2 => sprite.color = COLOR_DECAY2,
                        1 => sprite.color = COLOR_DECAY1,
                        _ => sprite.color = COLOR_DEAD,
                    }
                }
            }
        }
    }

    grid.num_alive = num_alive;

    //let elapsed = timer.elapsed().as_nanos();
    //let mut debug_text = debug_text.into_inner();
    //**debug_text = format!("Update time: {} ns\nAlive: {}", elapsed, num_alive);
}

fn mouse_button_input(
    buttons: Res<ButtonInput<MouseButton>>,
    window: Single<&Window, With<PrimaryWindow>>,
    camera_query: Single<(&Camera, &GlobalTransform)>,
    mut grid: ResMut<Grid>,
    mut current_tile: ResMut<CurrentTile>,
    mut query: Query<&mut Sprite, With<Cell>>,
    //time: Res<Time>,
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
                    // ignore repeated clicks on the same cell
                    //&& (current_tile.timer + 1.0) >= time.elapsed_secs()
                    let handle_click = if col == current_tile.col && row == current_tile.row {
                        false
                    } else {
                        true
                    };
                    if handle_click {
                        if let Some(cell) = grid.get(row, col) {
                            if let Ok(mut sprite) = query.get_mut(cell) {
                                // toggle this cell alive or dead
                                if grid.state[row][col] == CELL_LIFETIME {
                                    grid.state[row][col] = 0;
                                    sprite.color = COLOR_DEAD;
                                } else {
                                    grid.state[row][col] = CELL_LIFETIME;
                                    sprite.color = COLOR_ALIVE;
                                }
                            }
                            // store the current tile and time we clicked
                            current_tile.col = col;
                            current_tile.row = row;
                            //current_tile.timer = time.elapsed_secs();
                        }
                    }
                }
            }
        }
    }
}

fn keyboard_input(
    keys: Res<ButtonInput<KeyCode>>,
    state: Res<State<GameState>>,
    mut next_state: ResMut<NextState<GameState>>,
    state_button_q: Single<&Children, With<StateButton>>,
    mut state_button_text_q: Query<&mut Text>,
    mut tick_delay: ResMut<TickDelay>,
) {
    if keys.just_pressed(KeyCode::Space) {
        let state_button_children = state_button_q.into_inner();
        let mut text = state_button_text_q
            .get_mut(state_button_children[0])
            .unwrap();
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
    } else if keys.just_pressed(KeyCode::Minus) || keys.just_pressed(KeyCode::NumpadSubtract) {
        // speed up
        if tick_delay.0 >= 100 {
            tick_delay.0 -= 100;
        }
    } else if keys.just_pressed(KeyCode::Equal) || keys.just_pressed(KeyCode::NumpadAdd) {
        // slow down
        if tick_delay.0 <= 400 {
            tick_delay.0 += 100;
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
                    // 33% chance of being alive
                    let alive = rng.0.random_bool(0.33);
                    let value = if alive { CELL_LIFETIME } else { 0 };
                    grid.state[row][col] = value;
                    if let Some(cell) = grid.get(row as usize, col as usize) {
                        if let Ok(mut sprite) = cell_q.get_mut(cell) {
                            grid.state[row][col] = value;
                            if alive {
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
                    grid.state[row][col] = 0;
                    if let Some(cell) = grid.get(row as usize, col as usize) {
                        if let Ok(mut sprite) = cell_q.get_mut(cell) {
                            grid.state[row][col] = 0;
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

#[cfg(not(target_arch = "wasm32"))]
fn debug_text_update(
    input: Res<ButtonInput<KeyCode>>,
    mut overlay: ResMut<FpsOverlayConfig>,
    debug_text: Single<&mut Visibility, With<DebugText>>,
) {
    if input.just_pressed(KeyCode::F11) {
        overlay.enabled = !overlay.enabled;

        let mut debug_text_visibility = debug_text.into_inner();
        *debug_text_visibility = if overlay.enabled {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
    }
}

fn status_text_update(
    status_text: Single<&mut Text, With<StatusText>>,
    grid: Res<Grid>,
    tick_delay: Res<TickDelay>,
) {
    let delay_text = if tick_delay.0 == 0 {
        "None  ".to_string()
    } else {
        format!("{} ms", tick_delay.0)
    };
    let mut status_text = status_text.into_inner();
    **status_text = format!(
        "Tick Delay(-/+): {}  Number of Living Cells: {}",
        delay_text, grid.num_alive
    );
}
