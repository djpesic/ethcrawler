use chrono::prelude::*;
use serde_derive::{Deserialize, Serialize};
use serde_json::Value;
use sqlite::{Connection, ValueInto};
use std::{time::{SystemTime, UNIX_EPOCH}, collections::HashMap};
use tokio::sync::Mutex;


#[derive(Debug, Deserialize, Serialize, Clone, PartialEq, Eq)]
enum Direction{IN, OUT}
impl ValueInto for Direction{
    fn into(value: &sqlite::Value) -> Option<Self> {
        if let Some(val)=value.as_string() {
            if val=="IN" {Some(Direction::IN)} else {Some(Direction::OUT)}
        }else{
            None
        }
    }
}
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Transaction{
    address: String,
    direction: Direction,
    transfered: f64,
    transaction_fee: f64,
    timestamp:i64,
}
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Erc20Transaction{
    value: f64,
    token_symbol: String,
    address: String,
    contract_address: String,
    decimals:f64,
    timestamp:i64,
    direction: Direction,
}

pub struct Crawler{
    eth_scan:EtherScan,
    db:Mutex<Connection>,
}
impl Crawler{
    pub const BATCH_SIZE:i32=100;
    pub async fn new()->Self{
        
        let dbm = Mutex::new(sqlite::open("res/database.db").unwrap());
        Crawler{
            eth_scan:EtherScan::new(),
            db:dbm,
        }
    }
    pub async fn get_latest_block_number(&self)->String{
        self.eth_scan.get_latest_block_number().await
    }

    pub async fn get_number_of_batches(&self)->i32{
        let len = self.get_transaction_number().await;
        let number = len/ Crawler::BATCH_SIZE;
        let rem = len % Crawler::BATCH_SIZE;
        if rem == 0 {number} else {number+1}
    }
    async fn get_transaction_number(&self)->i32{
        let statement = "SELECT count(*) FROM transactions";
        let db = self.db.lock().await;
        let mut cursor = db.prepare(statement).unwrap().into_cursor();
        let mut len = 0;
        while let Some(Ok(row))=cursor.next(){
           len = row.get(0)
        }
        len as i32
    }
    pub async fn get_batch(&self, batch_index:i32)->Vec<Transaction>{
        let len = self.get_transaction_number().await;
        let start = Crawler::BATCH_SIZE * batch_index;
        let mut end =start+Crawler::BATCH_SIZE-1;
        if end > len-1 {
            end = len-1;
        }
        let statement = "SELECT * FROM transactions WHERE position>=? and position<=?";
        let db = self.db.lock().await;
        let vals = [sqlite::Value::Integer(start as i64), sqlite::Value::Integer(end as i64)];
        let mut cursor = db.prepare(statement).unwrap().into_cursor().bind(&vals).unwrap();
        let mut batch = Vec::new();
        while let Some(Ok(row))=cursor.next(){
            let t = Transaction{
                address:row.get(2),
                direction:row.get(3),
                transfered:row.get(4),
                transaction_fee:row.get(5),
                timestamp:row.get(6),
            };
            batch.push(t);
        }
        batch
    }
    pub async fn save_transactions(&mut self, transactions: Vec<Transaction>){
        let db = self.db.lock().await;
        db.execute("DROP TABLE transactions").unwrap();
        db.execute(
    "
                CREATE TABLE \"transactions\" (
                    \"id\"	INTEGER,
                    \"position\"	INTEGER,
                    \"address\"	TEXT,
                    \"direction\"	TEXT,
                    \"transfered\"	REAL,
                    \"fee\"	REAL,
                    \"timestamp\"   INTEGER,
                    PRIMARY KEY(\"id\")
                );
                "
        ).unwrap();
        let mut statement = String::from("INSERT INTO transactions (position, address, direction, transfered, fee, timestamp) VALUES");
        for i in 0..transactions.len(){
            let t = transactions.get(i).unwrap();
            let mut entry =format!("({},'{}','{:?}',{},{},{})",
            i,t.address, t.direction, t.transfered, t.transaction_fee, t.timestamp);
            if i<transactions.len()-1{
                entry.push(',');
            }
            statement.push_str(entry.as_str());
        }
        db.execute(statement).unwrap();
    }

    pub async fn get_transactions(&self, address: String, start_block_number:String, end_block_number:String)->Vec<Transaction>{
        let mut result = Vec::new();
        let mut page = 1;
        loop{
            let mut rsp = self.eth_scan.clone().get_list_of_normal_transactions(address.clone(), 
            start_block_number.clone(), end_block_number.clone(), page, 10000/page).await;
            page=page+1;
            if rsp.is_empty(){
                break;
            }
            result.append(&mut rsp);
        }
        println!{"Transactions are downloaded. Total number: {}",result.len()};
        result
    }

    async fn get_erc20_transactions(&self, address: String, start_block_number:String, end_block_number:String)->Vec<Erc20Transaction>{
        let mut result = Vec::new();
        let mut page = 1;
        loop{
            let mut rsp = self.eth_scan.clone().get_list_of_erc20_transactions(address.clone(), 
            start_block_number.clone(), end_block_number.clone(), page, 10000/page).await;
            page=page+1;
            if rsp.is_empty(){
                break;
            }
            result.append(&mut rsp);
        }
        println!{"Erc20 transactions are downloaded. Total number: {}",result.len()};
        result
    }

    pub async fn calculate_eth_balance(&self, time:String, address:String)->f64{
        let mut latest_block = self.eth_scan.get_latest_block_number().await;
        println!("Latest block: {}", latest_block);
        let mut latest_balance = self.eth_scan.get_ether_balance_for_address(address.clone()).await;
        println!("Latest balance: {}", latest_balance);
        let time = Utc.datetime_from_str(time.as_str(), "%Y-%m-%d %H:%M:%S").unwrap();
        let timestamp = time.timestamp();
        println!("Timestamp: {}", timestamp);
        loop{
            let start_block :i128= latest_block.parse().unwrap();
            let mut end_block = start_block - 100i128 * Crawler::BATCH_SIZE as i128;
            if end_block < 0 {end_block = 0;}
            let transactions = self.get_transactions(
                address.clone(), end_block.to_string(), start_block.to_string()).await;
            for t in transactions{
                if t.timestamp < timestamp{
                    return latest_balance;
                }
                latest_balance = if t.direction==Direction::IN {
                    latest_balance - t.transfered + t.transaction_fee
                }else{
                    latest_balance + t.transfered + t.transaction_fee
                }
            }
            if end_block ==0 {break;}
            latest_block = (end_block-1).to_string();
        }
        latest_balance
    }

    pub async fn calculate_erc20_balance(&self, time: String, address:String)->HashMap<String, f64>{
        let mut latest_block = self.eth_scan.get_latest_block_number().await;
        println!("Latest block: {}", latest_block);

        let time = Utc.datetime_from_str(time.as_str(), "%Y-%m-%d %H:%M:%S").unwrap();
        let timestamp = time.timestamp();
        println!("Timestamp: {}", timestamp);

        let mut transactions = Vec::new();
        let mut break_outer = false;
        loop{
            let start_block :i128= latest_block.parse().unwrap();
            let mut end_block = start_block - 100i128 * Crawler::BATCH_SIZE as i128;
            if end_block < 0 {end_block = 0;}
            let txs = self.get_erc20_transactions(
                address.clone(), end_block.to_string(), start_block.to_string()).await;
           
            for t in txs{
                if t.timestamp < timestamp{
                    break_outer=true;
                    break;
                }else{
                    transactions.push(t);
                }
                
            }
            if break_outer{break;}
            if end_block ==0 {break;}
            latest_block = (end_block-1).to_string();
        }
        
        transactions.sort_by(|t1, t2|{
            t1.token_symbol.cmp(&t2.token_symbol)
        });


        let mut result = HashMap::new();
        for t in transactions{
            match result.get_mut(&t.token_symbol){
                Some(v)=>{
                    *v= if t.direction == Direction::IN{
                        *v - t.value
                    }else{
                        *v + t.value
                    };
                },
                None=>{
                   
                    let latest_balance = 
                        self.eth_scan.get_erc20_balance_for_address(t.address.clone(),
                        t.contract_address.clone()).await;
                    let latest_balance = latest_balance.parse::<f64>().unwrap() / (10 as f64).powf(t.decimals);
                    let val = if t.direction == Direction::IN{
                        latest_balance - t.value
                    }else{
                        latest_balance + t.value
                    };
                    result.insert(t.token_symbol.clone(), val);

                }
            };
        }
       
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
    async fn get_list_of_normal_transactions(self,
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

    async fn get_list_of_erc20_transactions(self,
        address: String,
        startblock: String,
        endblock: String,
        page: u64,
        offset: u64
    )->Vec<Erc20Transaction>{
        let uri = format!{"{}?module=account&action=tokentx&address={}&startblock={}&endblock={}&page={}&offset={}&sort=asc&apikey={}",
    EtherScan::API_ROOT, address, startblock, endblock,page, offset, EtherScan::API_KEY};
        println!("{}", uri);
        let rsp = reqwest::get(uri).await.unwrap().text().await.unwrap();
        self.parse_erc20_transaction_list(rsp, address)
    }


    pub async fn get_latest_block_number(&self)->String{
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

    pub async fn get_ether_balance_for_address(&self, address: String)->f64{
        let uri = format!{"{}?module=account&action=balance&address={}&tag=latest&apikey={}",
            EtherScan::API_ROOT, address, EtherScan::API_KEY};
        println!("uri: {}",uri);
        let rsp = reqwest::get(uri).await.unwrap().text().await.unwrap();
        let parsed:Value = serde_json::from_str(&rsp).unwrap();
        let parsed = parsed["result"].as_str().unwrap();
        let result :f64= parsed.parse().unwrap();
        result / 1e18
    }

    async fn get_erc20_balance_for_address(&self, address: String, contract_address:String)->String{
        let uri = format!{"{}?module=account&action=tokenbalance&contractaddress={}&address={}&tag=latest&apikey={}",
            EtherScan::API_ROOT, contract_address, address, EtherScan::API_KEY};
        println!("uri: {}",uri);
        let rsp = reqwest::get(uri).await.unwrap().text().await.unwrap();
        let parsed:Value = serde_json::from_str(&rsp).unwrap();
        let parsed = parsed["result"].as_str().unwrap();
        parsed.to_owned()
    }

    fn parse_transaction_list(&self, rsp:String, address: String)->Vec<Transaction>{
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
            let timestamp = tr.get("timeStamp").unwrap().to_string();
            let timestamp:i64 = timestamp[1..timestamp.len()-1].parse().unwrap();
            result.push(Transaction { address: if addr_from==address {addr_to} else{addr_from}, 
                direction, 
                transfered: value, 
                transaction_fee: tx_fee,
                timestamp,
             });
        }   
        result
    }

    fn parse_erc20_transaction_list(&self, rsp:String, address: String)->Vec<Erc20Transaction>{
        let parsed:Value = serde_json::from_str(&rsp).unwrap();
     
        let result_arr = parsed["result"].as_array().unwrap();
        let mut result: Vec<Erc20Transaction> = Vec::new();
        for val in result_arr{
            let tr = val.as_object().unwrap();
            let value= tr.get("value").unwrap().to_string();
            let value:f64 = value[1..value.len()-1].parse().unwrap();
            let decimals=tr.get("tokenDecimal").unwrap().to_string();
            let decimals:f64 = decimals[1..decimals.len()-1].parse().unwrap();
            let value = value as f64 / (10 as f64).powf(decimals) as f64;
            let symbol = tr.get("tokenSymbol").unwrap().to_string();
            let symbol = symbol[1..symbol.len()-1].to_string();
            let contract_address = tr.get("contractAddress").unwrap().to_string();
            let contract_address = contract_address[1..contract_address.len()-1].to_string();
            let addr_to = tr.get("to").unwrap().to_string();
            let addr_to = addr_to[1..addr_to.len()-1].to_string();
            let addr_from = tr.get("from").unwrap().to_string();
            let addr_from = addr_from[1..addr_from.len()-1].to_string();
            let timestamp = tr.get("timeStamp").unwrap().to_string();
            let timestamp:i64 = timestamp[1..timestamp.len()-1].parse().unwrap();
            let direction = if addr_to.eq(&address){
                Direction::IN
            } else {
                Direction::OUT
            };
            result.push(Erc20Transaction { 
                value, token_symbol: symbol, 
                address: if addr_from==address {addr_to} else{addr_from},
                contract_address, 
                decimals, timestamp, direction});
        }   
        result
    }
}

