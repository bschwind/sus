# sus

An open-source clone of a popular impostor game.

## Dependencies
- cargo
- rustc
- cmake (for building GLSL shaders with `shaderc`)

## Build

```
$ cargo build --release
```

## Run the Game

```
$ cargo run --bin client --release
```

## Run the Server

```
$ cargo run --bin server --release
```

## Testing

```
$ cargo test
```

## Code Format

The formatting options currently use nightly-only options.

```
$ cargo +nightly fmt
```

## Code Linting

```
$ cargo clippy
```
