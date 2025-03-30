use axum::extract::Path;
use axum::http::header;
use axum::response;
use axum::{Json, Router, routing::get};
use base64::prelude::*;
use cached::proc_macro::cached;
use futures::future;
use rand::prelude::*;
use regex::Regex;
use rss::{ChannelBuilder, ItemBuilder};
use scraper::{Html, Selector};
use serde::Serialize;
use std::sync::LazyLock;
use std::time::{Duration as SysDuration, SystemTime};
use tokio::time::{self, Duration};
use tower_http::services::ServeDir;

#[tokio::main]
async fn main() {
    let ss = SofterSpeedScroller::new();
    let app = ss.get_service_future();
    let populate_caches = ss.get_cacher_future();

    tokio::join!(app, populate_caches);
}

struct SofterSpeedScroller {}

impl SofterSpeedScroller {
    fn new() -> Self {
        Self {}
    }
}

impl SpeedScroller for SofterSpeedScroller {}

trait SpeedScroller {
    async fn get_service_future(&self) {
        let app = Router::new()
            .route(
                "/getComic/{id}",
                get(async |Path(id)| get_comic(Some(id)).await),
            )
            .route("/getComic/", get(get_newest_comic))
            .route("/maxComicId", get(max_comic_id))
            .route("/rss.xml", get(rss_feed))
            // For compatibility
            .route("/rss.php", get(rss_feed))
            .fallback_service(ServeDir::new("static"));

        let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
        axum::serve(listener, app).await.unwrap();
    }

    async fn get_cacher_future(&self) {
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
    }
}

#[derive(Serialize, Clone)]
struct Comic {
    image_url: String,
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
async fn get_comic_data(id: Option<u32>) -> response::Result<Comic> {
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
    Ok(Comic {
        image: image_base64,
        title: comic_title,
        image_url: img_url,
    })
}

async fn get_comic(id: Option<u32>) -> response::Result<Json<Comic>> {
    Ok(axum::Json(get_comic_data(id).await?))
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

#[cached(
    sync_writes = "default",
    time = 36000,
    time_refresh = false,
    result = true
)]
async fn rss_feed() -> response::Result<(header::HeaderMap, String)> {
    static ONE_DAY: SysDuration = Duration::from_secs(24 * 60 * 60);
    static N_RSS_FEED_ITEMS: u64 = 10;
    let headers = {
        let mut base = header::HeaderMap::new();
        base.insert(header::CONTENT_TYPE, "text/html".parse().unwrap());
        base
    };
    let current_day = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap()
        .div_duration_f32(ONE_DAY) as u64;
    let max_comic_id: u64 = max_comic_id().await?.parse().unwrap();
    let futures = (0..N_RSS_FEED_ITEMS).map(async |days_ago| -> response::Result<_> {
        let current_day = current_day - days_ago;
        let mut rng = SmallRng::seed_from_u64(current_day);
        let comic_id = rng.random_range(1..=max_comic_id);
        let comic_data = get_comic_data(Some(comic_id as u32)).await?;
        Ok(ItemBuilder::default()
            .title(format!("{}", comic_id))
            .description(format!(
                r#"<a href="http://softerworld.casualvegetables.duckdns.org/?comic={idx}">
					<img src="{url}" />				</a> <br />
                                        {title}
					"#,
                idx = comic_id,
                url = comic_data.image_url,
                title = comic_data.title
            ))
            .link(format!(
                "https://www.asofterworld.com/index.php?id={idx}",
                idx = comic_id
            ))
            .build())
    });

    Ok((
        headers,
        format!(
            "{}",
            ChannelBuilder::default()
                .title("A Softer World")
                .link("http://softerworld.casualvegetables.duckdns.org")
                .description("A Softer World Comic")
                .language("us-en".to_string())
                .items(
                    future::try_join_all(futures)
                        .await
                        .map_err(|e| format!("{:?}", e))?
                )
                .namespaces([
                    (
                        "atom".to_string(),
                        "http://www.w3.org/2005/Atom".to_string()
                    ),
                    (
                        "dc".to_string(),
                        "http://purl.org/dc/elements/1.1/".to_string()
                    )
                ])
                .build()
        ),
    ))
}
