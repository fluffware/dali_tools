use crate::error::DynResult;
use crate::httpd::web_server::{self, ServerConfig};
use bytes::Bytes;
use rust_embed::RustEmbed;
use std::future::Future;
use std::net::IpAddr;

#[derive(RustEmbed)]
#[folder = "web"]
#[include = "*.xhtml"]
#[include = "*.html"]
#[include = "*.js"]
#[include = "*.css"]
#[include = "*.svg"]
#[include = "*.webm"]
#[include = "*.webp"]
#[include = "*.wav"]
struct WebFiles;

pub fn start(
    conf: ServerConfig,
) -> DynResult<(impl Future<Output = Result<(), hyper::Error>>, IpAddr, u16)> {
    let conf = conf.web_resource(Box::new(|path| {
        let mut path = path.trim_start_matches('/');
        if path.is_empty() {
            path = "index.html";
        }
        let suffix = path.rsplit('.').next().unwrap_or("");
        let mime_type = match suffix {
            "xhtml" => "application/xhtml+xml",
            "html" => "text/html",
            "hbs" => "text/x.handlebars",
            "js" => "text/javascript",
            "svg" => "image/svg+xml",
	    "webm" => "video/webm",
	    "webm" => "image/webp",
            "css" => "text/css",
            "wav" => "audio/wave",
            _ => "application/octet-stream",
        };
        match WebFiles::get(path) {
            Some(embedded) => Ok((mime_type, Bytes::from(embedded.data.into_owned()))),
            None => Err("Not found".into()),
        }
    }));

    let (server, bound_ip, bound_port) = web_server::setup_server(conf);

    return Ok((server, bound_ip, bound_port));
}
