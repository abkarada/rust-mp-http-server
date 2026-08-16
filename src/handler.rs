use std::fs::{self, File};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use crate::request::{Method, Request};
use crate::response::{self, Response};
use crate::router::Router;

static PUBLIC_DIRECTORY: OnceLock<PathBuf> = OnceLock::new();

/// Configures the public directory for static file serving.
pub fn set_public_directory<P: AsRef<Path>>(path: P) {
    let _ = PUBLIC_DIRECTORY.set(path.as_ref().to_path_buf());
}

pub fn get_public_directory() -> &'static Path {
    PUBLIC_DIRECTORY
        .get()
        .map(|p| p.as_path())
        .unwrap_or_else(|| Path::new("."))
}

pub fn create_router() -> Router {
    let mut router = Router::new();

    // GET /
    router.add_route(Method::Get, "/", |req, _params| {
        Response::ok(req)
    });

    // GET /echo/{str}
    router.add_route(Method::Get, "/echo/{str}", |req, params| {
        let echo_val = params.get("str").unwrap_or("");
        Response::ok(req).with_text_body(echo_val.to_string(), req)
    });

    // GET /user-agent
    router.add_route(Method::Get, "/user-agent", |req, _params| {
        let agent = req.headers.get("User-Agent").cloned().unwrap_or_default();
        Response::ok(req).with_text_body(agent, req)
    });

    // GET /files/{filename}
    router.add_route(Method::Get, "/files/{filename}", |req, params| {
        let filename = match params.get("filename") {
            Some(f) => f,
            None => return Response::not_found(),
        };

        let target_path = get_public_directory().join(filename);
        if !target_path.exists() || !target_path.is_file() {
            return Response::not_found();
        }

        match fs::read(&target_path) {
            Ok(content) => {
                let mime = response::mime_type_for_path(filename);
                Response::ok(req).with_typed_body(content, mime, req)
            }
            Err(_) => Response::not_found(),
        }
    });

    // POST /files/{filename}
    router.add_route(Method::Post, "/files/{filename}", |req, params| {
        let filename = match params.get("filename") {
            Some(f) => f,
            None => return Response::not_found(),
        };

        let target_path = get_public_directory().join(filename);
        let body_bytes = req.body.as_deref().unwrap_or(b"");

        match File::create(target_path) {
            Ok(mut file) => match file.write_all(body_bytes) {
                Ok(_) => Response::created(req),
                Err(_) => Response::create_error(req),
            },
            Err(_) => Response::create_error(req),
        }
    });

    // DELETE /files/{filename}
    router.add_route(Method::Delete, "/files/{filename}", |req, params| {
        let filename = match params.get("filename") {
            Some(f) => f,
            None => return Response::not_found(),
        };

        let target_path = get_public_directory().join(filename);
        if !target_path.exists() {
            return Response::not_found();
        }

        match fs::remove_file(target_path) {
            Ok(_) => Response::no_content(req),
            Err(_) => Response::create_error(req),
        }
    });

    router
}

static ROUTER: OnceLock<Router> = OnceLock::new();

pub fn route(req: &Request) -> Response {
    let router = ROUTER.get_or_init(create_router);
    router.handle(req)
}
