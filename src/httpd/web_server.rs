use crate::error::DynResult;
use bytes::Bytes;
use http_body_util::Full;
use hyper::Method;
use hyper::body::Incoming;
use hyper::header;
use hyper::http::StatusCode;
use hyper::server::conn::http1;
use hyper::service::service_fn;
use hyper::{Request, Response};
use hyper_util::rt::TokioIo;
#[allow(unused_imports)]
use log::{debug, error, info};
use std::future::Future;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::sync::{Arc, Mutex};
use tokio::net::TcpListener;

pub type BuildPage = Box<dyn FnMut(Request<Incoming>) -> DynResult<Response<Full<Bytes>>> + Send>;

/// Takes a path and returns (mime_type, resource_data)
pub type GetResource = Box<dyn FnMut(&str) -> DynResult<(&str, Bytes)> + Send>;

pub struct ServerConfig {
    bind_addr: Option<IpAddr>,
    port: Option<u16>,
    build_page: Option<BuildPage>,
    web_resource: GetResource,
}

fn no_resource(_path: &str) -> DynResult<(&str, Bytes)> {
    Err("No resource".into())
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

    pub fn web_resource(mut self, resource: GetResource) -> Self {
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

async fn handle(
    conf: Arc<Mutex<ServerConfig>>,
    req: Request<Incoming>,
) -> DynResult<Response<Full<Bytes>>> {
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
                        .body(Full::new(Bytes::from("No dynamic content".to_string())))
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
                                .body(Full::new(Bytes::from(format!("File error: {e}"))))
                                .map_err(|e| {
                                    Box::new(e) as Box<dyn std::error::Error + Send + Sync>
                                });
                        }
                    }
                };
                Response::builder()
                    .status(StatusCode::OK)
                    .header(header::CONTENT_TYPE, mime_type)
                    .body(Full::new(Bytes::from(data)))
                    .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)
            }
        }
        m => Response::builder()
            .status(StatusCode::METHOD_NOT_ALLOWED)
            .header(header::CONTENT_TYPE, "text/plain")
            .body(Full::new(Bytes::from(format!("Method {m} not supported"))))
            .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>),
    }
}
pub async fn setup_server(
    conf: ServerConfig,
    cancel: impl Future<Output = ()>,
) -> DynResult<(impl Future<Output = DynResult<()>>, IpAddr, u16)> {
    let port = conf.port.unwrap_or(0);
    let bind_addr = conf
        .bind_addr
        .unwrap_or_else(|| IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)));
    let socket_addr = SocketAddr::new(bind_addr, port);
    let conf = Arc::new(Mutex::new(conf));
    let listener = TcpListener::bind(&socket_addr).await?;
    let port = listener.local_addr().unwrap().port();
    let addr = listener.local_addr().unwrap().ip();
    let graceful = hyper_util::server::graceful::GracefulShutdown::new();
    let http = http1::Builder::new();
    let mut cancel = Box::pin(cancel);
    let server = async move {
        loop {
            let conf_clone = conf.clone();
            #[rustfmt::skip]
            tokio::select! {
		Ok((stream, _)) = listener.accept() => {
                    let io = TokioIo::new(stream);
                    let http_conn =
			http.serve_connection(io, service_fn(move |req| {
			    handle(conf_clone.clone(), req)
			}));
                    let run = graceful.watch(http_conn);
                    tokio::task::spawn(async move {
			if let Err(err) = run.await {
			    error!("Error serving connection: {:?}", err);
			}
                    });
		}
		() = &mut cancel => {
                    drop(listener);
                    break;
		}
            }
        }
        Ok(())
    };
    Ok((server, addr, port))
}
