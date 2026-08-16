# High Performance Multi-Reactor HTTP Server

A multi-threaded HTTP server written in Rust using `mio` and `httparse`.

## Features

- Master-Worker Multi-Reactor event loop
- HTTP/1.1 parsing with `httparse`
- Non-blocking socket I/O
- Idle connection timeouts
- Dynamic route matching (`/echo/{str}`, `/files/{filename}`)

## Usage

Build the project:
```sh
cargo build --release
```

Run the server:
```sh
cargo run --release -- --directory ./public
```

Run tests:
```sh
cargo test
```
