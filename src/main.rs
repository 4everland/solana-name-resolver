#[macro_use]
extern crate lazy_static;

use std::convert::Infallible;
use std::net::SocketAddr;
use std::string::String;
use std::time::Duration;

use hyper::{Body, Request, Response};
use hyper::server::conn::Http;
use hyper::service::service_fn;
use moka::sync::Cache;
use tokio::net::TcpListener;

pub use derivation::get_name_url;

mod constants;
mod derivation;
mod utils;

lazy_static! {
static ref RESOLVE_CACHE: Cache<String, String> = Cache::builder().time_to_live(Duration::from_secs(3600)).build();
}

#[tokio::main]
pub async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let addr: SocketAddr = ([0, 0, 0, 0], 8080).into();

    let listener = TcpListener::bind(addr).await?;
    println!("Listening on http://{}", addr);
    loop {
        let (stream, _) = listener.accept().await?;

        tokio::task::spawn(async move {
            if let Err(err) = Http::new()
                .serve_connection(stream, service_fn(handler))
                .await
            {
                println!("Error serving connection: {:?}", err);
            }
        });
    }
}

pub async fn handler(r: Request<Body>) -> Result<Response<Body>, Infallible> {
    let host = r.headers().get(hyper::header::HOST).unwrap().to_str().unwrap();

    let name = match host.find(":") {
        None => host,
        Some(i) => &host[0..i]
    }.trim_end_matches(constants::GATEWAY_DOMAIN);

    let target = match RESOLVE_CACHE.get(name) {
        None => {
            match get_name_url(name).await {
                Err(_) => String::from(constants::ERROR_URL),
                Ok(t) => t.to_string()
            }
        }
        Some(t) => t.to_string()
    };

    if String::is_empty(&target) {
        RESOLVE_CACHE.insert(String::from(name), target.clone())
    }

    Ok(Response::builder()
        .status(302)
        .header(hyper::header::LOCATION,
                target.as_str().trim_end_matches("/").to_owned() + r.uri().to_string().as_str())
        .body(Body::empty()).unwrap())
}


