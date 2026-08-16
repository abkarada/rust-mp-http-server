# Rust Multi-Protocol HTTP Server

An experimental HTTP server written in Rust to explore non-blocking I/O, event-driven server architectures, connection management, and HTTP protocol implementation.

The server uses `mio` for OS-level event polling and distributes TCP connections across worker reactors. It includes HTTP/1.1 handling, experimental HTTP/2 support, TLS integration, and an experimental UDP-based implementation used to explore HTTP/3 framing concepts.

This is a systems programming project intended for learning and experimentation rather than production use.

## Features

- **HTTP/1.1:** Request parsing and response handling over TCP using `httparse`.
- **Experimental HTTP/2:** Binary framing, HPACK header compression, stream handling, and basic flow-control bookkeeping.
- **HTTP/3 Exploration:** Experimental UDP-based framing and header encoding inspired by HTTP/3 and QPACK concepts. This is not a complete QUIC or RFC-compliant HTTP/3 implementation.
- **TLS 1.3:** TLS support using `rustls`, certificate generation with `rcgen`, and ALPN negotiation for `h2` and `http/1.1`.
- **Multi-Reactor Architecture:** Connections are distributed across worker event loops using `mio::Poll`.
- **Connection Ownership:** Accepted sockets are transferred to worker reactors through channels and workers manage their own connection state.
- **Non-Blocking I/O:** Read and write operations handle `WouldBlock` without blocking worker threads.
- **Buffered Partial Writes:** Responses that cannot be written completely are retained and resumed when the socket becomes writable.
- **Reusable Connection Buffers:** Connection state uses `BytesMut` buffers for incremental reads and parsing.
- **HTTP/2 Flow Control:** Basic stream- and connection-level window tracking and `WINDOW_UPDATE` handling.
- **Idle Connection Cleanup:** Inactive connections are periodically removed.
- **Dynamic Routing:** Supports path parameters such as `/echo/{str}` and `/files/{filename}`.
- **Static File Operations:** Basic file serving and upload endpoints.
- **Gzip Compression:** Responses can be compressed when requested through `Accept-Encoding`.

## Architecture

The server follows a master/worker reactor model.

The main reactor accepts incoming TCP connections and distributes them across worker reactors. Each worker owns its assigned connections and processes readiness events independently through `mio::Poll`.

```text
                              ┌──────────────────────────┐
                              │      Main Reactor        │
                              │                          │
                              │  TCP Listener            │
                              │  UDP Experiment Socket   │
                              └────────────┬─────────────┘
                                           │
                              TCP connection distribution
                                           │
                    ┌──────────────────────┴──────────────────────┐
                    │                                             │
                    ▼                                             ▼
          ┌─────────────────────┐                     ┌─────────────────────┐
          │   Worker Reactor 0  │                     │   Worker Reactor N  │
          │                     │                     │                     │
          │   mio::Poll         │                     │   mio::Poll         │
          │   Connection State  │                     │   Connection State  │
          │   Read/Write Buffer │                     │   Read/Write Buffer │
          └──────────┬──────────┘                     └──────────┬──────────┘
                     │                                           │
                     └───────────────────┬───────────────────────┘
                                         │
                                         ▼
                              ┌──────────────────────────┐
                              │     Router / Handler     │
                              └──────────────────────────┘
```

## Design Notes

### Non-Blocking I/O

The server uses `mio` to work with OS readiness notifications rather than dedicating a blocking thread to each connection.

When an operation returns `WouldBlock`, the connection remains registered with the event loop and processing continues when the socket becomes ready again.

### Partial Writes and Backpressure

A socket is not guaranteed to accept an entire response in a single write.

When only part of a response can be transmitted, the remaining bytes are retained in the connection state. The connection is registered for writable readiness and transmission resumes when the socket can accept more data.

This provides basic backpressure handling without blocking the worker reactor.

### Connection Distribution

The main reactor accepts TCP connections and transfers ownership to worker reactors.

Workers receive connections through channels and are notified through `mio::Waker`. After assignment, connection state and I/O are handled by the corresponding worker.

### HTTP/2

The HTTP/2 implementation is experimental and focuses on understanding several parts of the protocol, including:

- binary frame parsing
- HPACK header compression
- stream state
- connection-level flow control
- stream-level flow control
- `WINDOW_UPDATE` processing

It should not be considered a complete production HTTP/2 implementation.

### HTTP/3 Experiment

The repository also contains an experimental UDP-based implementation used to explore HTTP/3-style framing and header encoding.

It does **not** implement the complete QUIC transport protocol and should not be interpreted as an RFC-compliant HTTP/3 server.

A complete implementation would require substantially more functionality, including QUIC connection management, cryptographic packet protection, stream management, acknowledgements, loss recovery, congestion control, and other protocol requirements.

## Supported Endpoints

| Method | Path | Description |
|---|---|---|
| `GET` | `/` | Returns a basic `200 OK` response |
| `GET` | `/echo/{str}` | Returns the supplied path value as text |
| `GET` | `/user-agent` | Returns the client's `User-Agent` header |
| `GET` | `/files/{filename}` | Serves a file from the configured directory |
| `POST` | `/files/{filename}` | Writes the request payload to a file |

Responses can also use gzip compression when the client advertises support through `Accept-Encoding`.

## Command-Line Options

| Argument | Description | Default |
|---|---|---|
| `--directory <path>` / `-d <path>` | Directory used for file operations | `./` |
| `--help` | Displays usage information | — |

## Building

### Requirements

- Rust toolchain
- Cargo

Clone the repository:

```sh
git clone https://github.com/abkarada/rust-mp-http-server.git
cd rust-mp-http-server
```

Build a release binary:

```sh
cargo build --release
```

Run the server:

```sh
cargo run --release -- --directory ./public
```

Run the test suite:

```sh
cargo test
```

## Packaging

The repository contains packaging files for running the server as a Linux service.

### Arch Linux

A `PKGBUILD` is included for building an Arch Linux package locally.

```sh
makepkg -si
```

### Debian / Ubuntu

A Debian package can be built locally using `cargo-deb`:

```sh
cargo deb
sudo dpkg -i target/debian/high-performance-http-server_0.1.0_amd64.deb
```

### systemd

When installed with the provided service configuration, the server can be managed using systemd:

```sh
sudo systemctl start mp-http-server
sudo systemctl enable mp-http-server
sudo systemctl status mp-http-server
```

## Project Scope

This project was built primarily to explore:

- event-driven network programming in Rust
- OS-level readiness polling
- multi-reactor server architectures
- connection state management
- non-blocking socket I/O
- backpressure and partial writes
- HTTP/1.1 parsing
- HTTP/2 framing and flow control
- TLS and ALPN
- HTTP/3 and QUIC protocol concepts

It is an experimental side project and is not intended to replace production HTTP servers or provide complete RFC-compliant implementations of every protocol explored in the repository.

## License

Distributed under the MIT License. See [LICENSE](LICENSE) for details.
