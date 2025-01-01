use crate::error::DynResult;
use bytes::Bytes;
use futures_util::sink::SinkExt;
use futures_util::StreamExt;
use hyper::header;
use hyper::http::StatusCode;
use hyper::service::{make_service_fn, service_fn};
use hyper::Method;
use hyper::{Body, Request, Response, Server};
use hyper_websocket_lite::AsyncClient;
#[allow(unused_imports)]
use log::{debug, error, info};
use std::convert::Infallible;
use std::future::Future;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::sync::{Arc, Mutex};
use tokio::sync::{broadcast, mpsc};
use websocket_lite::{Message, Opcode};

pub type BuildPage = Box<dyn FnMut(Request<Body>) -> DynResult<Response<Body>> + Send>;

/// Takes a path and returns (mime_type, resource_data)
pub type GetResurce = Box<dyn FnMut(&str) -> DynResult<(&str, Bytes)> + Send>;

pub struct ServerConfig {
    bind_addr: Option<IpAddr>,
    port: Option<u16>,
    build_page: Option<BuildPage>,
    web_resource: GetResurce,
    ws: Option<(WsSender, WsReceiveChannel)>,
}

fn no_resource(_path: &str) -> DynResult<(&str, Bytes)> {
    Err("No rosurce".into())
}
impl ServerConfig {
    pub fn new() -> Self {
        Self {
            bind_addr: None,
            port: None,
            build_page: None,
            web_resource: Box::new(no_resource),
            ws: None,
        }
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

    pub fn web_socket(mut self, ws_send: WsSender, ws_receive: WsReceiveChannel) -> Self {
        self.ws = Some((ws_send, ws_receive));
        self
    }
}

type WsSender = mpsc::Sender<Bytes>;
type WsReceiver = broadcast::Receiver<Bytes>;
type WsReceiveChannel = broadcast::Sender<Bytes>;

pub async fn ws_client(mut client: AsyncClient, ws_send: WsSender, mut ws_receive: WsReceiver) {
    info!("Connected WS");
    loop {
        tokio::select! {
            res = client.next() => {
                if let Some(msg) = res {
                    if let Ok(msg) = msg {
                        if msg.opcode() == Opcode::Text {
                            if let Err(e) = ws_send.send(msg.into_data()).await {
                                error!("Failed to send WS message to handler: {}",e)
                            }
                        }
                    }
                } else {
                    break;
                }
            }
            Ok(data) = ws_receive.recv() => {
                match Message::new(Opcode::Text, data) {
                    Ok(msg) => {
                        if let Err(e) = client.send(msg).await {
                            error!("Failed to send message to WS client: {}",e);
                        }
                    }
                    Err(e) => {
                        error!("Failed to create message to WS client: {}",e);
                    }
                }


            }
        }
    }
    info!("Client disconnected")
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
                    return Response::builder()
                        .status(StatusCode::NOT_FOUND)
                        .header(header::CONTENT_TYPE, "text/plain")
                        .body(Body::from(format!("No dynamic content")))
                        .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>);
                }
            } else if path.starts_with("/socket/") {
                let (ws_send, ws_receive) = {
                    let mut conf = conf.lock().unwrap();
                    if let Some((ws_send, ws_receive)) = &mut conf.ws {
                        (ws_send.clone(), ws_receive.subscribe())
                    } else {
                        return Response::builder()
                            .status(StatusCode::NOT_FOUND)
                            .header(header::CONTENT_TYPE, "text/plain")
                            .body(Body::from(format!("Web Socket not enabled")))
                            .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>);
                    }
                };
                hyper_websocket_lite::server_upgrade(req, |client| {
                    ws_client(client, ws_send, ws_receive)
                })
                .await
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
                                })
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
