# Bevy Life

An implementation of Conway's Game of Life built with [Bevy](https://bevyengine.org/)

\
Cells update according to the classic Life rules, with recently dead cells fading through a short decay trail. The grid wraps at the edges, so cells on one side of the window can neighbor cells on the opposite side.

## Features

- Interactive grid editing with the mouse
- Run/pause controls
- Randomize and clear buttons
- Adjustable tick delay
- Optional FPS/update-time debug overlay
- Parallel row updates using Bevy's compute task pool

## Build

```
cargo build --release
```

## Run

```
cargo run --release
```
