use eth_crawler::crawler::Crawler;
use warp::Filter;
use std::{fs, collections::HashMap,  convert::Infallible};

#[tokio::main]
async fn main() {
    
    
    let main_page = warp::path!("crawler")
        .map(||{warp::reply::html(fs::read_to_string("res/main_page.html").unwrap())});

    let get_transactions = warp::path!("crawler" / "get_transactions").
    and(warp::body::form()).
    and_then(get_transactions);

    let number_of_batches = warp::path!("crawler" / "number_of_batches").
    and(warp::get()).
    and_then(get_number_of_batches);

    let get_batch = warp::path!("crawler" / "get_batch").
    and(warp::body::form()).
    and_then(get_batch);

    let historical_balance = warp::path!("crawler" / "historical_balance").
    and(warp::body::form()).
    and_then(get_historical_balance);
    
    let final_routes = 
    main_page.or(get_transactions).or(number_of_batches).or(get_batch).or(historical_balance);

    warp::serve(final_routes)
        .run(([127, 0, 0, 1], 3030))
        .await;
}

async fn get_transactions(simple_map:HashMap<String, String>)->Result<impl warp::Reply, Infallible>{
    let mut crawler = Crawler::new().await;
    let block_number = simple_map.get("block_number").unwrap().into();
    let address = simple_map.get("address").unwrap().into();
    println!("Getting transaction data for address {}, block number {}", address, block_number);
    let endblock = crawler.get_latest_block_number().await;
    println!("endblock: {}",endblock);
    let rsp = crawler.get_transactions(address, block_number, endblock).await;
    crawler.save_transactions(rsp).await;
    Ok(warp::reply::json(&(crawler.get_batch(0).await)))
}

async fn get_historical_balance(simple_map:HashMap<String, String>)->Result<impl warp::Reply, Infallible>{
    let crawler = Crawler::new().await;
    let date = simple_map.get("date").unwrap().into();
    let address = simple_map.get("address").unwrap().into();
    println!("Getting historical balance for address {}, date {}", address, date);
    let balance = crawler.calculate_eth_balance(date, address).await;
    Ok(warp::reply::json(&balance))
}


async fn get_batch(simple_map:HashMap<String, String>)->Result<impl warp::Reply, Infallible>{
    let crawler = Crawler::new().await;
    let batch_number:i32 = simple_map.get("batch_number").unwrap().to_owned().parse().unwrap();
    let batch_index = batch_number-1;
    println!("Getting transaction batch number {}", batch_number);
    let rsp = crawler.get_batch(batch_index).await;
    Ok(warp::reply::json(&rsp))
}

async fn get_number_of_batches()->Result<impl warp::Reply, Infallible>{
    let crawler = Crawler::new().await;
    let batch_number = crawler.get_number_of_batches().await;
    println!("Number of transaction batches: {}", batch_number);
    Ok(warp::reply::json(&batch_number))
}