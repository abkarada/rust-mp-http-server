# High Performance Multi-Protocol HTTP Server

A multi-threaded, zero-copy HTTP/1.1, HTTP/2, and HTTP/3 (QUIC) server written in Rust using `mio`, `httparse`, `hpack`, `rustls`, and `bytes`.

## Why Rust & Why This Project?

Even though mature HTTP servers like Nginx are incredible software, they still suffer from C/C++ memory safety vulnerabilities in 2026 (buffer overflows, use-after-free bugs, dangling pointers, and memory-related CVEs). Rust was chosen for this project to eliminate 99% of memory safety errors at compile-time without sacrificing bare-metal performance.

This is not a massive enterprise project—it is a side-fun project built to explore high-concurrency network programming and modern HTTP protocol implementations.

## Features

- **Multi-Protocol Engine:** Supports HTTP/1.1 (TCP), HTTP/2 (TCP / Binary Framing / HPACK), and HTTP/3 (UDP / QUIC / QPACK).
- **TLS 1.3 Encryption:** Memory-safe TLS 1.3 encryption using `rustls`, self-signed cert generation (`rcgen`), and ALPN negotiation (`h2`, `http/1.1`).
- **Master-Worker Multi-Reactor:** Multi-threaded event loop per CPU core (`mio::Poll`).
- **Zero Lock Contention:** Socket ownership is transferred via lock-free channels (`mpsc`) and `mio::Waker` signals.
- **Zero-Allocation Buffer Pooling:** `bytes::BytesMut` connection buffers for $O(1)$ zero-copy slicing and heap fragmentation prevention.
- **Zero-Copy Parsing:** HTTP/1.1 headers parsed directly over raw byte slices with `httparse`.
- **Non-Blocking Write State Machine:** Handles socket backpressure (`WouldBlock`) asynchronously.
- **HTTP/2 Flow Control:** Stream and connection-level `WINDOW_UPDATE` window tracking.
- **Slowloris & Timeout Protection:** Idle connection cleanup.
- **Dynamic Pattern Router:** Supports dynamic path parameters (`/echo/{str}`, `/files/{filename}`) and `405 Method Not Allowed`.

## Quick Installation

### 1-Line Cloud VM Installer
Run this command on any Linux Cloud VM (Ubuntu, Debian, Arch, Fedora, Alpine):

```sh
curl -fsSL https://raw.githubusercontent.com/abkarada/rust-mp-http-server/main/install.sh | sh
```

### Systemd Service Management

Start the server:
```sh
sudo systemctl start mp-http-server
```

Enable on boot:
```sh
sudo systemctl enable mp-http-server
```

Check status:
```sh
sudo systemctl status mp-http-server
```

### Arch Linux (AUR) Installation
Build and install via AUR:

```sh
yay -S rust-mp-http-server
```

### Debian / Ubuntu (.deb) Package Generation
Build a `.deb` package using `cargo-deb`:

```sh
cargo deb
sudo dpkg -i target/debian/high-performance-http-server_0.1.0_amd64.deb
```

## Manual Building

Build from source:
```sh
cargo build --release
```

Run the binary:
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
