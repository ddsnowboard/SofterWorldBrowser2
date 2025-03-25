use axum::extract::Path;
use axum::{Json, Router, routing::get};
use base64::prelude::*;
use cached::proc_macro::cached;
use regex::Regex;
use scraper::{Html, Selector};
use serde::Serialize;
use std::sync::LazyLock;
use tower_http::services::ServeDir;

#[tokio::main]
async fn main() {
    // build our application with a route
    let app = Router::new()
        .route(
            "/getComic/{id}",
            get(async |Path(id)| get_comic(Some(id)).await),
        )
        .route("/getComic/", get(get_newest_comic))
        .route("/maxComicId", get(max_comic_id))
        .fallback_service(ServeDir::new("static"));

    // run our app with hyper, listening globally on port 3000
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

#[derive(Serialize, Clone)]
struct Comic {
    image: String,
    title: String,
}

async fn get_newest_comic() -> Json<Comic> {
    get_comic(None).await
}

#[cached(sync_writes = "by_key", time = 36000, time_refresh = false)]
async fn get_comic(id: Option<u32>) -> Json<Comic> {
    let (comic_title, img_url) = {
        let parsed_html = get_comic_page(id).await;
        let comic_img_tag = {
            let div_selector = Selector::parse("div#comicimg").unwrap();
            let img_selector = Selector::parse("img").unwrap();
            parsed_html
                .select(&div_selector)
                .next()
                .unwrap()
                .select(&img_selector)
                .next()
                .unwrap()
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
    axum::Json(Comic {
        image: image_base64,
        title: comic_title,
    })
}

static ID_FROM_LINK_REGEX: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"asofterworld\.com/index\.php\?id=(\d+)").unwrap());
async fn get_comic_page(id: Option<u32>) -> Html {
    let url = match id {
        Some(id) => format!("https://www.asofterworld.com/index.php?id={}", id),
        None => "https://www.asofterworld.com/index.php".to_string(),
    };
    let page_text = reqwest::get(url).await.unwrap().text().await.unwrap();
    Html::parse_document(&page_text)
}

#[cached(sync_writes = "default", time = 36000, time_refresh = false)]
async fn max_comic_id() -> String {
    let parsed_html = get_comic_page(None).await;
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
    format!("{}", index + 1)
}
