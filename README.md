# Bevy Life

An implementation of [Conway's Game of Life](https://en.wikipedia.org/wiki/Conway%27s_Game_of_Life) built with [Bevy](https://bevyengine.org/)

Cells update according to the classic Life rules, with recently dead cells fading through a short decay trail. The grid wraps at the edges, so cells on one side of the window can neighbor cells on the opposite side.

## Build

```
cargo build --release
```

## Run

```
cargo run --release
```
