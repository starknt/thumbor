use std::{
    collections::hash_map::DefaultHasher,
    hash::{Hash, Hasher},
    net::SocketAddr,
    num::NonZeroUsize,
    sync::Arc,
};

use anyhow::Result;
use axum::{
    extract::Path,
    http::{HeaderMap, HeaderValue, StatusCode},
    response::Html,
    routing::get,
    Extension, Router,
};

mod engine;
mod pb;
use bytes::Bytes;
use engine::{Engine, Photon};
use lru::LruCache;
use pb::*;
use percent_encoding::{percent_encode, NON_ALPHANUMERIC};
use serde::Deserialize;
use tokio::sync::Mutex;
use tower::ServiceBuilder;
use tower_http::add_extension::AddExtensionLayer;
use tracing::info;

async fn index_handler() -> Html<&'static str> {
    "Hello, World!".into()
}

#[derive(Debug, Deserialize)]
struct Params {
    spec: String,
    url: String,
}

type Cache = Arc<Mutex<LruCache<u64, Bytes>>>;

#[tokio::main]
async fn main() -> Result<()> {
    // 初始化 tracing
    tracing_subscriber::fmt::init();
    let cap = NonZeroUsize::new(1024).unwrap();
    let cache: Cache = Arc::new(Mutex::new(LruCache::new(cap)));

    let app = Router::new()
        .route("/", get(index_handler))
        .route("/image/:spec/:url", get(generate))
        .layer(
            ServiceBuilder::new()
                .layer(AddExtensionLayer::new(cache))
                .into_inner(),
        );

    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
    let mut fd = listenfd::ListenFd::from_env();
    let listener = match fd.take_tcp_listener(0).unwrap() {
        Some(listener) => tokio::net::TcpListener::from_std(listener).unwrap(),
        None => tokio::net::TcpListener::bind(addr).await.unwrap(),
    };

    tracing::debug!("listening on {}", addr);
    print_test_url("https://img.xjh.me/random_img.php?return=302&time=13145");
    print_test_url2("https://img.xjh.me/random_img.php?return=302&time=13145");

    axum::serve(listener, app).await.unwrap();

    Ok(())
}

async fn generate(
    Path(Params { spec, url }): Path<Params>,
    Extension(cache): Extension<Cache>,
) -> Result<(HeaderMap, Vec<u8>), StatusCode> {
    let url = percent_encoding::percent_decode_str(&url).decode_utf8_lossy();
    let spec: ImageSpec = spec
        .as_str()
        .try_into()
        .map_err(|_| StatusCode::BAD_REQUEST)?;

    let data = retrieve_image(&url, cache)
        .await
        .map_err(|_| StatusCode::BAD_REQUEST)?;

    let mut engine: Photon = data
        .try_into()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    engine.apply(&spec.specs);

    let image = engine.generate(image::ImageOutputFormat::Png);

    info!("Finished processing: img size {}", image.len());

    let mut headers = HeaderMap::new();
    headers.insert("content-type", HeaderValue::from_static("image/png"));

    Ok((headers, image))
}

async fn retrieve_image(url: &str, cache: Cache) -> Result<Bytes> {
    let mut hasher = DefaultHasher::new();
    url.hash(&mut hasher);
    let key = hasher.finish();

    let g = &mut cache.lock().await;
    let data = match g.get(&key) {
        Some(v) => {
            info!("Match cache {}", key);
            v.to_owned()
        }
        None => {
            info!("Retrieve from url: {}", url);
            let resp = reqwest::get(url).await?;
            let data = resp.bytes().await?;
            g.put(key, data.clone());
            data
        }
    };

    Ok(data)
}

fn print_test_url(url: &str) {
    use std::borrow::Borrow;
    let spec1 = Spec::new_resize(500, 800, resize::SampleFilter::CatmullRom);
    let spec2 = Spec::new_watermark(20, 20, 1);
    let spec3 = Spec::new_filter(filter::Filter::Marine);
    let image_spec = ImageSpec::new(vec![spec1, spec2, spec3]);
    let s: String = image_spec.borrow().into();
    let test_image = percent_encode(url.as_bytes(), NON_ALPHANUMERIC).to_string();
    println!("test url: http://localhost:3000/image/{}/{}", s, test_image);
}

fn print_test_url2(url: &str) {
    use std::borrow::Borrow;
    let spec1 = Spec::new_oil();
    let spec2 = Spec::new_watermark(20, 20, 1);
    let spec3 = Spec::new_filter(filter::Filter::Marine);
    let spec4 = Spec::new_draw_text("Hello World".to_string(), 40, 40);
    let image_spec = ImageSpec::new(vec![spec1, spec2, spec3, spec4]);
    let s: String = image_spec.borrow().into();
    let test_image = percent_encode(url.as_bytes(), NON_ALPHANUMERIC).to_string();
    println!("test url: http://localhost:3000/image/{}/{}", s, test_image);
}
