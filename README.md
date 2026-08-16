# High Performance Multi-Protocol HTTP Server

A multi-threaded, zero-copy HTTP/1.1, HTTP/2, and HTTP/3 (QUIC) server written in Rust using `mio`, `httparse`, `hpack`, and custom QUIC/QPACK parsers.

## Why Rust & Why This Project?

Even though mature HTTP servers like Nginx are incredible software, they still suffer from C/C++ memory safety vulnerabilities in 2026 (buffer overflows, use-after-free bugs, dangling pointers, and memory-related CVEs). Rust was chosen for this project to eliminate 99% of memory safety errors at compile-time without sacrificing bare-metal performance.

This is not a massive enterprise project—it is a side-fun project built to explore high-concurrency network programming and modern HTTP protocol implementations.

## Features

- **Multi-Protocol Engine:** Supports HTTP/1.1 (TCP), HTTP/2 (TCP / Binary Framing / HPACK), and HTTP/3 (UDP / QUIC / QPACK).
- **Master-Worker Multi-Reactor:** Multi-threaded event loop per CPU core (`mio::Poll`).
- **Zero Lock Contention:** Socket ownership is transferred via lock-free channels (`mpsc`) and `mio::Waker` signals.
- **Zero-Copy Parsing:** HTTP/1.1 headers parsed directly over raw byte slices with `httparse`.
- **Non-Blocking Write State Machine:** Handles socket backpressure (`WouldBlock`) asynchronously.
- **Slowloris & Timeout Protection:** Idle connection cleanup.
- **Dynamic Pattern Router:** Supports dynamic path parameters (`/echo/{str}`, `/files/{filename}`) and `405 Method Not Allowed`.

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

## Contact

- **Author:** Abdurrahman Karadağ
- **Email:** abdurrahman.karadag@roftcore.com
