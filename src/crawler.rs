use serde_derive::{Deserialize, Serialize};
use serde_json::Value;
use sqlite::{Connection, Statement, ValueInto};
use std::{time::{SystemTime, UNIX_EPOCH}, fmt::format};
use tokio::sync::Mutex;


#[derive(Debug, Deserialize, Serialize, Clone)]
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
                    PRIMARY KEY(\"id\")
                );
                "
        ).unwrap();
        let mut statement = String::from("INSERT INTO transactions (position, address, direction, transfered, fee) VALUES");
        for i in 0..transactions.len(){
            let t = transactions.get(i).unwrap();
            let mut entry =format!("({},'{}','{:?}',{},{})",i,t.address, t.direction, t.transfered, t.transaction_fee);
            if i<transactions.len()-1{
                entry.push(',');
            }
            statement.push_str(entry.as_str());
        }
        db.execute(statement).unwrap();
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

