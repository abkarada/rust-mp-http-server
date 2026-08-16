use std::env;

mod compression;
mod error;
mod handler;
mod http2;
mod http3;
mod request;
mod response;
mod router;
mod server;
mod tls;

fn main() {
    // Parse --directory CLI argument safely if provided
    let args: Vec<String> = env::args().collect();
    for i in 0..args.len() {
        if (args[i] == "--directory" || args[i] == "-d") && i + 1 < args.len() {
            handler::set_public_directory(&args[i + 1]);
            break;
        }
    }

    let srv = server::Server::new("127.0.0.1:4221").expect("failed to bind");
    srv.run();
}
