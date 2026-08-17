# Rust Multi-Protocol HTTP Server

A performance-oriented, event-driven HTTP server written in Rust.

The server is built around a multi-reactor architecture using `mio`, with a main reactor responsible for accepting connections and multiple worker reactors responsible for connection I/O and protocol processing.

The implementation includes HTTP/1.1, selected HTTP/2 mechanisms, TLS 1.3 integration, dynamic routing, connection lifecycle management, and an experimental UDP-based implementation for HTTP/3 framing concepts.

The project focuses on low-level network server architecture and explicit control over connection ownership, event polling, buffering, protocol state, and backpressure.

## Architecture

The server follows a master/worker multi-reactor model.

A main reactor owns the listening sockets and accepts incoming TCP connections. Accepted connections are distributed across worker reactors in round-robin order.

Each worker runs an independent `mio::Poll` event loop and owns the complete state of the connections assigned to it.

```text
                         ┌─────────────────────────┐
                         │      Main Reactor       │
                         │                         │
                         │     TCP Listener        │
                         │     UDP Socket          │
                         └────────────┬────────────┘
                                      │
                           Connection Dispatch
                             (Round-Robin)
                                      │
                  ┌───────────────────┴───────────────────┐
                  │                                       │
                  ▼                                       ▼
        ┌─────────────────────┐                 ┌─────────────────────┐
        │   Worker Reactor 0  │                 │   Worker Reactor N  │
        │                     │                 │                     │
        │     mio::Poll       │                 │     mio::Poll       │
        │   Connection State  │                 │   Connection State  │
        │   Read/Write Buffers│                 │   Read/Write Buffers│
        └──────────┬──────────┘                 └──────────┬──────────┘
                   │                                       │
                   └───────────────────┬───────────────────┘
                                       │
                                       ▼
                            ┌─────────────────────┐
                            │  Protocol Handling  │
                            │   Router / Handler  │
                            └─────────────────────┘
```

The number of worker reactors is derived from the parallelism available on the host.

This keeps connection processing distributed across independent event loops while avoiding shared ownership of active connection state between workers.

## Features

### Event-Driven Networking

- Non-blocking TCP I/O using `mio`
- OS-level readiness polling
- Independent worker event loops
- Per-connection protocol and buffer state
- Dynamic readable/writable interest registration
- Idle connection cleanup

### Multi-Reactor Execution

The main reactor accepts incoming TCP connections and transfers them to worker reactors.

Workers are notified using `mio::Waker` and subsequently register the new connection with their own polling instance.

After assignment, the worker owns the connection state and handles its I/O lifecycle independently.

### HTTP/1.1

The HTTP/1.1 path includes:

- request parsing with `httparse`
- persistent connection handling
- dynamic routing
- path parameters
- request header processing
- response generation
- static file operations
- gzip response compression
- connection lifecycle handling

### HTTP/2

The repository contains an HTTP/2 implementation covering several core protocol mechanisms, including:

- HTTP/2 connection preface detection
- binary frame processing
- HPACK header compression
- stream state
- stream-level flow control
- connection-level flow control
- `WINDOW_UPDATE` handling
- response framing

The implementation covers a subset of HTTP/2 and should not be considered a complete implementation of the HTTP/2 specification.

### HTTP/3 / UDP Experiment

The repository also contains an experimental UDP-based protocol path used to implement and examine HTTP/3-style framing and header encoding.

This component is not a complete QUIC implementation and therefore is not an RFC-compliant HTTP/3 server.

A complete HTTP/3 implementation would additionally require the full QUIC transport machinery, including connection establishment, packet protection, stream management, acknowledgements, loss recovery, congestion control, and related transport behavior.

### TLS

TLS support is implemented using `rustls`.

The TLS layer includes:

- TLS 1.3
- certificate generation using `rcgen`
- ALPN negotiation
- HTTP/1.1 and HTTP/2 protocol selection

## Non-Blocking I/O

Connections are processed through readiness notifications rather than through a blocking thread-per-connection model.

Each worker maintains its own collection of active connections.

When a socket becomes readable, the worker consumes available input until the operation would block. The accumulated bytes are stored in a per-connection `BytesMut` buffer and passed to the appropriate protocol implementation.

```text
Socket Ready
     │
     ▼
Read Available Bytes
     │
     ▼
Connection Buffer
     │
     ▼
Protocol Detection / Parsing
     │
     ▼
Request Handler
     │
     ▼
Response Buffer
     │
     ▼
Non-Blocking Write
```

If the socket cannot currently accept more data, processing returns to the event loop rather than blocking the worker thread.

## Backpressure and Partial Writes

Socket writes are not assumed to complete in a single operation.

Each connection maintains a write buffer containing pending response data.

If only part of the buffer can be written, the transmitted bytes are removed and the connection remains registered for writable readiness.

```text
Response
   │
   ▼
Write Buffer
   │
   ├──── socket accepts all data ────► READABLE
   │
   └──── partial / WouldBlock
                 │
                 ▼
          retain remaining data
                 │
                 ▼
        READABLE | WRITABLE
                 │
                 ▼
          resume transmission
```

Once the pending output has been transmitted, writable interest is removed until it is needed again.

## Connection Distribution

Accepted TCP connections are assigned to worker reactors using round-robin distribution.

```text
Connection 0  ──► Worker 0
Connection 1  ──► Worker 1
Connection 2  ──► Worker 2
...
Connection N  ──► Worker N
Connection N+1 ─► Worker 0
```

Connection ownership is transferred through channels, while `mio::Waker` is used to notify the target worker that new work is available.

This design keeps active connection state local to the worker responsible for processing it.

## Protocol Detection

New TCP connections initially have an unknown protocol state.

Incoming data is inspected to distinguish HTTP/1.1 traffic from the HTTP/2 connection preface.

The connection then transitions into the corresponding protocol state:

```text
             ┌─────────────┐
             │   Unknown   │
             └──────┬──────┘
                    │
            inspect input bytes
                    │
           ┌────────┴────────┐
           │                 │
           ▼                 ▼
      HTTP/1.1            HTTP/2
           │                 │
           ▼                 ▼
      HTTP/1 State      HTTP/2 State
```

Protocol-specific state is retained for the lifetime of the connection.

## Routing

The server contains a small dynamic router supporting static and parameterized paths.

Examples include:

```text
/
 /echo/{str}
 /user-agent
 /files/{filename}
```

### Supported Endpoints

| Method | Path | Description |
|---|---|---|
| `GET` | `/` | Returns a basic `200 OK` response |
| `GET` | `/echo/{str}` | Returns the supplied path value |
| `GET` | `/user-agent` | Returns the client's `User-Agent` header |
| `GET` | `/files/{filename}` | Serves a file from the configured directory |
| `POST` | `/files/{filename}` | Writes the request payload to a file |

Responses may also be gzip-compressed when the client advertises support through `Accept-Encoding`.

## Connection Lifecycle

Each connection tracks its most recent activity.

Inactive connections are periodically removed after the configured idle timeout.

Connection cleanup includes deregistering the socket from the worker reactor and removing its associated protocol and buffer state.

## Project Structure

```text
src/
├── main.rs
├── server.rs
├── request.rs
├── response.rs
├── router.rs
├── handler.rs
├── http2.rs
├── http3.rs
├── tls.rs
├── compression.rs
└── error.rs
```

The major responsibilities are separated between server/reactor management, HTTP protocol processing, routing, response generation, TLS, and compression.

## Building

### Requirements

- Rust toolchain
- Cargo
- Linux, macOS, or another platform supported by the underlying `mio` polling mechanisms

Clone the repository:

```sh
git clone https://github.com/abkarada/rust-mp-http-server.git
cd rust-mp-http-server
```

Build the release binary:

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

## Command-Line Options

| Argument | Description | Default |
|---|---|---|
| `--directory <path>` / `-d <path>` | Directory used for file operations | `./` |
| `--help` | Displays usage information | — |

## Linux Packaging

The repository includes files for packaging and running the server as a Linux service.

### Arch Linux

A `PKGBUILD` is included and can be built locally with:

```sh
makepkg -si
```

### Debian / Ubuntu

A Debian package can be generated using `cargo-deb`:

```sh
cargo deb
sudo dpkg -i target/debian/high-performance-http-server_0.1.0_amd64.deb
```

### systemd

A systemd service definition is also included.

```sh
sudo systemctl start mp-http-server
sudo systemctl enable mp-http-server
sudo systemctl status mp-http-server
```

## Scope and Limitations

The server implements a performance-oriented architecture, but this repository does not currently publish a reproducible performance benchmark suite.

For that reason, the architecture should not by itself be interpreted as a claim of specific throughput, latency, or performance relative to established HTTP servers.

Protocol completeness also differs between implementations:

- HTTP/1.1 is the primary protocol path.
- HTTP/2 implements a subset of the protocol and its flow-control mechanisms.
- The HTTP/3 component is experimental and does not implement the complete QUIC transport protocol.

The project is not intended as a drop-in replacement for mature production servers such as Nginx, Apache, Caddy, or established Rust HTTP stacks.

Its focus is the implementation of the server architecture itself: event-driven I/O, reactor management, connection ownership, buffering, protocol state, and HTTP processing with relatively little abstraction between the application and the underlying networking primitives.

## License

Distributed under the MIT License. See [LICENSE](LICENSE) for details.
