use warp::Filter;
use std::fs;

#[tokio::main]
async fn main() {
    // GET /crawler => 200 OK with body main_page.html
    let main_page = warp::path!("crawler")
        .map(||{warp::reply::html(fs::read_to_string("res/main_page.html").unwrap())});

    warp::serve(main_page)
        .run(([127, 0, 0, 1], 3030))
        .await;
}