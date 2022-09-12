# ethcrawler

## Description

An application will allow a user to view transaction data from the Ethereum blockchain associated with a specific wallet address W that the user inputs, starting with block B. The application get information on:

* wallets (addresses) and 

* amounts of ETH associated with transactions made to and from the given wallet W and

* show them in a web page. 

The application collects and displays all transaction data starting from the given block B. 

## Instalation and running

You have to have Rust installed on the computer.
Compile:
```
cargo build
```
Application has a client and a server. 
To run server, open separate terminal and execute
```
cargo run
```
Client is tested in Chrome browser. Open Chrome and go to http://localhost:3030/crawler or http://127.0.0.1:3030/crawler.

Client web page have following inputs: Address (Eth address format), block number, date. For example: 0xaa7a9ca87d3694b5755f213b5d04094b8d0f0a6f, 15443116.

Button "Get transactions" start crawling process. Results will be displayed inside table, and separated by pages. Table supports navigation:
* Next - Go to the next page.
* Prev - Go to the previous page.
* Go - Jump to the specified page. Page number should be entered in text box.

Button "Get historical balance" starts calculation of ETH balance for given address and given time.