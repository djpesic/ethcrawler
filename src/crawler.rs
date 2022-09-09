use serde_derive::{Deserialize, Serialize};
use serde_json::Value;
use std::time::{SystemTime, UNIX_EPOCH};


#[derive(Debug, Deserialize, Serialize, Clone)]
enum Direction{IN, OUT}
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Transaction{
    address: String,
    direction: Direction,
    transfered: f64,
    transaction_fee: f64,
}


#[derive(Debug)]
pub struct Crawler{
    eth_scan:EtherScan,
    transactions: Vec<Transaction>,
}
impl Crawler{
    pub const BATCH_SIZE:i32=100;
    pub fn new()->Self{
        Crawler{
            eth_scan:EtherScan::new(),
            transactions:Vec::new(),
        }
    }
    pub fn get_number_of_batches(&self)->i32{
        let number = self.transactions.len() as i32/ Crawler::BATCH_SIZE;
        let rem = self.transactions.len() as i32 % Crawler::BATCH_SIZE;
        (number + rem) as i32
    }
    pub fn get_batch(&self, batch_index:i32)->Vec<Transaction>{
        let len = self.transactions.len() as i32;
        let start = Crawler::BATCH_SIZE * batch_index;
        let mut end =start+Crawler::BATCH_SIZE-1;
        if end > len-1 {
            end = len-1;
        }
        let batch:Vec<Transaction> = self.transactions.iter().enumerate().filter(|(index, _)|{
            (*index as i32>=start) && (*index as i32<=end)
        }).map(|(_,val)|{val.clone()}).collect();
        batch
    }
    pub fn save_transactions(&mut self, transactios: Vec<Transaction>){
        self.transactions = transactios;
    }
    pub async fn get_transactions(&self, address: String, block_number:String)->Vec<Transaction>{
        let mut result = Vec::new();
        let mut page = 1;
        let endblock = self.eth_scan.clone().get_latest_block_number().await;
        println!("endblock: {}",endblock);
        loop{
            let mut rsp = self.eth_scan.clone().get_list_of_normal_transactions(address.clone(), 
                block_number.clone(), endblock.clone(), page, 10000/page).await;
            page=page+1;
            if rsp.is_empty(){
                break;
            }
            result.append(&mut rsp);
        }
        println!{"Transactions are downloaded. Total number: {}",result.len()};
        result
    }
}
#[derive(Debug, Clone)]
struct EtherScan{

}
impl EtherScan{
    pub const API_KEY: &'static str = "XNRXJ8E9VRRRJ89KAQ3VVAEP447388YTHE";
    pub const API_ROOT: &'static str = "https://api.etherscan.io/api";
    pub fn new()->Self{
        EtherScan { }
    }
    pub async fn get_list_of_normal_transactions(self,
        address: String,
        startblock: String,
        endblock: String,
        page: u64,
        offset: u64
    )->Vec<Transaction>{
        let uri = format!{"{}?module=account&action=txlist&address={}&startblock={}&endblock={}&page={}&offset={}&sort=asc&apikey={}",
    EtherScan::API_ROOT, address, startblock, endblock,page, offset, EtherScan::API_KEY};
        println!("{}", uri);
        let rsp = reqwest::get(uri).await.unwrap().text().await.unwrap();
        self.parse_transaction_list(rsp, address)
    }

    pub async fn get_latest_block_number(self)->String{
        let time = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();
        let uri = format!{"{}?module=block&action=getblocknobytime&timestamp={}&closest=before&apikey={}",
            EtherScan::API_ROOT, time, EtherScan::API_KEY};
        println!("uri: {}",uri);
        let rsp = reqwest::get(uri).await.unwrap().text().await.unwrap();
        let parsed:Value = serde_json::from_str(&rsp).unwrap();
        let parsed = parsed["result"].as_str().unwrap();
        parsed.to_string()
    }   

    fn parse_transaction_list(self, rsp:String, address: String)->Vec<Transaction>{
        let parsed:Value = serde_json::from_str(&rsp).unwrap();
     
        let result_arr = parsed["result"].as_array().unwrap();
        let mut result: Vec<Transaction> = Vec::new();
        for val in result_arr{
            let tr = val.as_object().unwrap();
            let addr_to = tr.get("to").unwrap().to_string();
            let addr_to = addr_to[1..addr_to.len()-1].to_string();
            let addr_from = tr.get("from").unwrap().to_string();
            let addr_from = addr_from[1..addr_from.len()-1].to_string();
            let value= tr.get("value").unwrap().to_string();
            let value:u64 = value[1..value.len()-1].parse().unwrap();
            let gas_used = tr.get("gasUsed").unwrap().to_string();
            let gas_used:u64 = gas_used[1..gas_used.len()-1].parse().unwrap();
            let gas_price = tr.get("gasPrice").unwrap().to_string();
            let gas_price:u64 = gas_price[1..gas_price.len()-1].parse().unwrap();
            let direction = if addr_to.eq(&address){
                Direction::IN
            } else {
                Direction::OUT
            };
            let tx_fee = gas_price as f64/1e18*gas_used as f64;
            let value = value as f64 / 1e18 as f64;
            result.push(Transaction { address: if addr_from==address {addr_to} else{addr_from}, 
                direction, 
                transfered: value, 
                transaction_fee: tx_fee,
             });
        }   
        result
    }
}

