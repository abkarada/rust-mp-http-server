use std::collections::HashMap;
use std::sync::Arc;

use crate::request::{Method, Request};
use crate::response::Response;

/// Extracted route path parameters, e.g. `{"filename": "foo.txt"}`
#[derive(Debug, Default, Clone)]
pub struct Params {
    map: HashMap<String, String>,
}

impl Params {
    pub fn get(&self, key: &str) -> Option<&str> {
        self.map.get(key).map(|s| s.as_str())
    }

    pub fn insert(&mut self, key: String, val: String) {
        self.map.insert(key, val);
    }
}

/// A segment in a path pattern, e.g. "echo" or "{str}"
#[derive(Debug, Clone)]
enum Segment {
    Static(String),
    Param(String),
    Wildcard,
}

#[derive(Debug, Clone)]
pub struct PathPattern {
    segments: Vec<Segment>,
}

impl PathPattern {
    pub fn parse(pattern: &str) -> Self {
        let segments = pattern
            .split('/')
            .filter(|s| !s.is_empty())
            .map(|s| {
                if s.starts_with('{') && s.ends_with('}') {
                    let param_name = &s[1..s.len() - 1];
                    Segment::Param(param_name.to_string())
                } else if s == "*" {
                    Segment::Wildcard
                } else {
                    Segment::Static(s.to_string())
                }
            })
            .collect();

        Self { segments }
    }

    pub fn matches(&self, path: &str) -> Option<Params> {
        let path_segments: Vec<&str> = path.split('/').filter(|s| !s.is_empty()).collect();

        if self.segments.is_empty() && path_segments.is_empty() {
            return Some(Params::default());
        }

        if self.segments.len() != path_segments.len() {
            let is_wildcard = self
                .segments
                .last()
                .map_or(false, |s| matches!(s, Segment::Wildcard));
            if !is_wildcard || path_segments.len() < self.segments.len() - 1 {
                return None;
            }
        }

        let mut params = Params::default();

        for (pattern_seg, path_seg) in self.segments.iter().zip(path_segments.iter()) {
            match pattern_seg {
                Segment::Static(expected) => {
                    if expected != path_seg {
                        return None;
                    }
                }
                Segment::Param(name) => {
                    params.insert(name.clone(), path_seg.to_string());
                }
                Segment::Wildcard => {}
            }
        }

        Some(params)
    }
}

type HandlerFn = Arc<dyn Fn(&Request, &Params) -> Response + Send + Sync>;

pub struct Route {
    pub method: Method,
    pub pattern: PathPattern,
    pub handler: HandlerFn,
}

pub struct Router {
    routes: Vec<Route>,
}

impl Router {
    pub fn new() -> Self {
        Self { routes: Vec::new() }
    }

    pub fn add_route<F>(&mut self, method: Method, pattern_str: &str, handler: F)
    where
        F: Fn(&Request, &Params) -> Response + Send + Sync + 'static,
    {
        self.routes.push(Route {
            method,
            pattern: PathPattern::parse(pattern_str),
            handler: Arc::new(handler),
        });
    }

    pub fn handle(&self, req: &Request) -> Response {
        let mut path_matched_but_wrong_method = false;

        for route in &self.routes {
            if let Some(params) = route.pattern.matches(&req.path) {
                if route.method == req.method {
                    return (route.handler)(req, &params);
                } else {
                    path_matched_but_wrong_method = true;
                }
            }
        }

        if path_matched_but_wrong_method {
            Response::method_not_allowed()
        } else {
            Response::not_found()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pattern_matching() {
        let pattern = PathPattern::parse("/echo/{str}");
        let matched = pattern.matches("/echo/hello-world");
        assert!(matched.is_some());
        assert_eq!(matched.unwrap().get("str"), Some("hello-world"));
    }

    #[test]
    fn test_router_method_not_allowed() {
        let mut router = Router::new();
        router.add_route(Method::Get, "/files/{filename}", |req, _params| {
            Response::ok(req)
        });

        use std::str::FromStr;
        let req = Request::from_str("POST /files/foo.txt HTTP/1.1\r\n\r\n").unwrap();
        let res = router.handle(&req);
        assert_eq!(res.status_code, 405);
    }
}
