use axum::extract::Path;
use axum::{Json, Router, routing::get};
use base64::prelude::*;
use cached::proc_macro::cached;
use scraper::{Html, Selector};
use serde::Serialize;
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

#[cached(sync_writes = "by_key", time = 3600, time_refresh = false)]
async fn get_comic(id: Option<u32>) -> Json<Comic> {
    let url = match id {
        Some(id) => format!("https://www.asofterworld.com/index.php?id={}", id),
        None => "https://www.asofterworld.com/index.php".to_string(),
    };
    let page_text = reqwest::get(url).await.unwrap().text().await.unwrap();

    let (comic_title, img_url) = {
        let parsed_html = Html::parse_document(&page_text);
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

#[cached(sync_writes = "default", time = 3600, time_refresh = false)]
async fn max_comic_id() -> &'static str {
    "505"
}
