use warp::Filter;
use std::{fs, collections::HashMap};

#[tokio::main]
async fn main() {
    

    let main_page = warp::path!("crawler")
        .map(||{warp::reply::html(fs::read_to_string("res/main_page.html").unwrap())});

    let get_transactions = warp::path!("crawler" / "get_transactions").
    and(warp::body::form()).
    map(|simple_map: HashMap<String, String>|{
        println!("Received: {:?}", simple_map);
        ""
    });
    
    let final_routes = main_page.or(get_transactions);

    warp::serve(final_routes)
        .run(([127, 0, 0, 1], 3030))
        .await;
}