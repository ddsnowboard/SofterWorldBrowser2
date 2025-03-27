use axum::extract::Path;
use axum::response;
use axum::{Json, Router, routing::get};
use base64::prelude::*;
use cached::proc_macro::cached;
use regex::Regex;
use scraper::{Html, Selector};
use serde::Serialize;
use std::sync::LazyLock;
use tokio::time::{self, Duration};
use tower_http::services::ServeDir;

#[tokio::main]
async fn main() {
    let app = Router::new()
        .route(
            "/getComic/{id}",
            get(async |Path(id)| get_comic(Some(id)).await),
        )
        .route("/getComic/", get(get_newest_comic))
        .route("/maxComicId", get(max_comic_id))
        .fallback_service(ServeDir::new("static"));

    let app = async {
        let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
        axum::serve(listener, app).await.unwrap();
    };

    let populate_caches = async {
        // This waits until the caches area almost empty to start repopulating them.
        let mut cache_ttl = time::interval(Duration::from_secs(36000));
        let mut rate_limit = time::interval(Duration::from_secs(3));
        loop {
            cache_ttl.tick().await;
            println!("Refreshing cache...");
            let max_comic_id: u32 = match max_comic_id().await {
                Ok(id_string) => id_string.parse().unwrap(),
                Err(ref e) => {
                    println!("Error in cache refreshing: {:?}", e);
                    continue;
                }
            };
            for id in 1..=max_comic_id {
                rate_limit.tick().await;
                let _ = get_comic(Some(id)).await;
            }
            println!("Refreshed cache.");
        }
    };

    tokio::join!(app, populate_caches);
}

#[derive(Serialize, Clone)]
struct Comic {
    image: String,
    title: String,
}

async fn get_newest_comic() -> response::Result<Json<Comic>> {
    get_comic(None).await
}

#[cached(
    sync_writes = "by_key",
    time = 36000,
    time_refresh = false,
    result = true
)]
async fn get_comic(id: Option<u32>) -> response::Result<Json<Comic>> {
    let stringify_id = || id.map(|id| format!("{}", id)).unwrap_or("None".to_string());
    let (comic_title, img_url) = {
        let parsed_html = get_comic_page(id).await?;
        let comic_img_tag = {
            let div_selector = Selector::parse("div#comicimg").unwrap();
            let img_selector = Selector::parse("img").unwrap();
            parsed_html
                .select(&div_selector)
                .next()
                .ok_or(format!("Could not find div for id {}", stringify_id()))?
                .select(&img_selector)
                .next()
                .ok_or(format!("Could not find img for id {}", stringify_id()))?
                .value()
        };
        let comic_title = comic_img_tag.attr("title").unwrap().to_string();
        let image_url = comic_img_tag.attr("src").unwrap().to_string();
        (comic_title, image_url)
    };
    let image_base64 = {
        let image_blob = reqwest::get(&img_url).await.unwrap().bytes().await.unwrap();
        BASE64_STANDARD.encode(image_blob)
    };
    Ok(axum::Json(Comic {
        image: image_base64,
        title: comic_title,
    }))
}

async fn get_comic_page(id: Option<u32>) -> response::Result<Html> {
    let url = match id {
        Some(id) => format!("https://www.asofterworld.com/index.php?id={}", id),
        None => "https://www.asofterworld.com/index.php".to_string(),
    };
    let page_text = reqwest::get(url)
        .await
        .map_err(|e| format!("{:?}", e))?
        .text()
        .await
        .map_err(|e| format!("{:?}", e))?;
    Ok(Html::parse_document(&page_text))
}

#[cached(
    sync_writes = "default",
    time = 36000,
    time_refresh = false,
    result = true
)]
async fn max_comic_id() -> response::Result<String> {
    static ID_FROM_LINK_REGEX: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"asofterworld\.com/index\.php\?id=(\d+)").unwrap());
    let parsed_html = get_comic_page(None).await?;
    let div_selector = Selector::parse("div#previous").unwrap();
    let link_selector = Selector::parse("a").unwrap();
    let link_tag = parsed_html
        .select(&div_selector)
        .next()
        .unwrap()
        .select(&link_selector)
        .next()
        .unwrap()
        .value();
    let link = link_tag.attr("href").unwrap();
    let index: u32 = ID_FROM_LINK_REGEX
        .captures(link)
        .unwrap()
        .get(1)
        .unwrap()
        .as_str()
        .parse()
        .unwrap();
    Ok(format!("{}", index + 1))
}
