use eth_crawler::crawler::Crawler;
use tokio::sync::oneshot;
use warp::{Filter, get};
use std::{fs, collections::HashMap, hash::Hash, convert::Infallible};

#[tokio::main]
async fn main() {
    
    
    let main_page = warp::path!("crawler")
        .map(||{warp::reply::html(fs::read_to_string("res/main_page.html").unwrap())});

    let get_transactions = warp::path!("crawler" / "get_transactions").
    and(warp::body::form()).
    and_then(get_transactions);
    
    let final_routes = main_page.or(get_transactions);

    warp::serve(final_routes)
        .run(([127, 0, 0, 1], 3030))
        .await;
}

async fn get_transactions(simple_map:HashMap<String, String>)->Result<impl warp::Reply, Infallible>{
    let mut crawler = Crawler::new().await;
    let block_number = simple_map.get("block_number").unwrap().into();
    let address = simple_map.get("address").unwrap().into();
    println!("Getting transaction data for address {}, block number {}", address, block_number);
    let rsp = crawler.get_transactions(address, block_number).await;
    crawler.save_transactions(rsp).await;
    Ok(warp::reply::json(&(crawler.get_batch(0).await)))
}