use crate::error::DynResult;
use bytes::Bytes;
use hyper::Method;
use hyper::header;
use hyper::http::StatusCode;
use hyper::service::{make_service_fn, service_fn};
use hyper::{Body, Request, Response, Server};
#[allow(unused_imports)]
use log::{debug, error, info};
use std::convert::Infallible;
use std::future::Future;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::sync::{Arc, Mutex};

pub type BuildPage = Box<dyn FnMut(Request<Body>) -> DynResult<Response<Body>> + Send>;

/// Takes a path and returns (mime_type, resource_data)
pub type GetResurce = Box<dyn FnMut(&str) -> DynResult<(&str, Bytes)> + Send>;

pub struct ServerConfig {
    bind_addr: Option<IpAddr>,
    port: Option<u16>,
    build_page: Option<BuildPage>,
    web_resource: GetResurce,
}

fn no_resource(_path: &str) -> DynResult<(&str, Bytes)> {
    Err("No rosurce".into())
}
impl ServerConfig {
    pub fn new() -> Self {
	Self::default()
    }

    pub fn port(mut self, p: u16) -> Self {
        self.port = Some(p);
        self
    }
    pub fn bind_addr(mut self, a: IpAddr) -> Self {
        self.bind_addr = Some(a);
        self
    }

    pub fn build_page(mut self, f: BuildPage) -> Self {
        self.build_page = Some(f);
        self
    }

    pub fn web_resource(mut self, resource: GetResurce) -> Self {
        self.web_resource = resource;
        self
    }
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            bind_addr: None,
            port: None,
            build_page: None,
            web_resource: Box::new(no_resource),
        }
    }
}

async fn handle(conf: Arc<Mutex<ServerConfig>>, req: Request<Body>) -> DynResult<Response<Body>> {
    let path = req.uri().path();
    match req.method() {
        &Method::GET => {
            if path.starts_with("/dyn/") {
                let mut conf = conf.lock().unwrap();
                if let Some(build_page) = &mut conf.build_page {
                    build_page(req)
                } else {
                    Response::builder()
                        .status(StatusCode::NOT_FOUND)
                        .header(header::CONTENT_TYPE, "text/plain")
                        .body(Body::from("No dynamic content".to_string()))
                        .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)
                }
            } else {
                let (mime_type, data) = {
                    let mut conf = conf.lock().unwrap();
                    match (conf.web_resource)(req.uri().path()) {
                        Ok(res) => res,
                        Err(e) => {
                            return Response::builder()
                                .status(StatusCode::NOT_FOUND)
                                .header(header::CONTENT_TYPE, "text/plain")
                                .body(Body::from(format!("File error: {e}")))
                                .map_err(|e| {
                                    Box::new(e) as Box<dyn std::error::Error + Send + Sync>
                                });
                        }
                    }
                };
                Response::builder()
                    .status(StatusCode::OK)
                    .header(header::CONTENT_TYPE, mime_type)
                    .body(Body::from(data))
                    .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)
            }
        }
        m => Response::builder()
            .status(StatusCode::METHOD_NOT_ALLOWED)
            .header(header::CONTENT_TYPE, "text/plain")
            .body(Body::from(format!("Method {m} not supported")))
            .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>),
    }
}
pub fn setup_server(
    conf: ServerConfig,
) -> (impl Future<Output = Result<(), hyper::Error>>, IpAddr, u16) {
    let port = conf.port.unwrap_or(0);
    let bind_addr = conf
        .bind_addr
        .unwrap_or_else(|| IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)));
    let socket_addr = SocketAddr::new(bind_addr, port);
    let conf = Arc::new(Mutex::new(conf));
    let make_service = make_service_fn(move |_conn| {
        let conf = conf.clone();
        async move { Ok::<_, Infallible>(service_fn(move |req| handle(conf.clone(), req))) }
    });
    let server = Server::bind(&socket_addr).serve(make_service);
    let port = server.local_addr().port();
    let addr = server.local_addr().ip();
    (server, addr, port)
}
