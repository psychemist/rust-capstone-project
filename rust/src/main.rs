#![allow(unused)]
use bitcoin::hex::DisplayHex;
use bitcoin::psbt::raw;
use bitcoin::Address;
use bitcoincore_rpc::bitcoin::{Amount, Network};
use bitcoincore_rpc::{Auth, Client, RpcApi};
use serde::Deserialize;
use serde_json::json;
use std::fs::File;
use std::io::Write;

// Node access params
const RPC_URL: &str = "http://127.0.0.1:18443"; // Default regtest RPC port
const RPC_USER: &str = "alice";
const RPC_PASS: &str = "password";

// You can use calls not provided in RPC lib API using the generic `call` function.
// An example of using the `send` RPC call, which doesn't have exposed API.
// You can also use serde_json `Deserialize` derivation to capture the returned json result.
fn send(rpc: &Client, addr: &str) -> bitcoincore_rpc::Result<String> {
    let args = [
        json!([{addr : 100 }]), // recipient address
        json!(null),            // conf target
        json!(null),            // estimate mode
        json!(null),            // fee rate in sats/vb
        json!(null),            // Empty option object
    ];

    #[derive(Deserialize)]
    struct SendResult {
        complete: bool,
        txid: String,
    }

    let send_result = rpc.call::<SendResult>("send", &args)?;
    assert!(send_result.complete);
    Ok(send_result.txid)
}

fn main() -> bitcoincore_rpc::Result<()> {
    // Connect to Bitcoin Core RPC
    let rpc = Client::new(
        RPC_URL,
        Auth::UserPass(RPC_USER.to_owned(), RPC_PASS.to_owned()),
    )?;

    // Get blockchain info (RPC confirmation)
    let blockchain_info = rpc.get_blockchain_info()?;
    println!("Blockchain Info: {:?}", blockchain_info);

    // Create/Load the wallets, named 'Miner' and 'Trader'.
    let wallets = ["Miner", "Trader"];
    let loaded_wallets = rpc.list_wallets().unwrap();

    // Iterate through wallets array
    for wallet_name in wallets {
        // If wallet exists and is already loaded, continue through loop
        if loaded_wallets.contains(&wallet_name.to_string()) {
            println!("Wallet '{}' is already loaded", wallet_name);
            continue;
        } else {
            // Else if not loaded, try and load wallet via RPC call
            match rpc.load_wallet(wallet_name) {
                Ok(wallet_load_result) => {
                    println!("Wallet loaded: {:?}", wallet_load_result.name);
                }
                Err(e) => {
                    // Else, create wallet via RPC call
                    match rpc.create_wallet(wallet_name, None, None, None, None) {
                        Ok(wallet_create_result) => {
                            println!("Wallet created: {:?}", wallet_create_result.name);
                        }
                        Err(error) => {
                            println!("Wallet create error: {:?}", error);
                        }
                    }
                }
            }
        }
    }
    println!("Loaded wallets: {:?}", rpc.list_wallets().unwrap());

    // Generate spendable balances in the Miner wallet.
    let miner_rpc = Client::new(
        format!("{RPC_URL}/wallet/{}", wallets[0]).as_str(),
        Auth::UserPass(RPC_USER.to_owned(), RPC_PASS.to_owned()),
    )?;
    // println!(
    //     "Miner Wallet Info: {:?}",
    //     miner_rpc.get_wallet_info().unwrap()
    // );

    // Check if addresses are in wallet and use the first one, else create new address
    let addresses = miner_rpc
        .list_received_by_address(None, None, None, None)
        .unwrap();
    let miner_address = if addresses.is_empty() {
        miner_rpc
            .get_new_address(Some("Mining Reward"), None)
            .unwrap()
    } else {
        addresses[0].address.clone()
    };

    // Mine new blocks to address in miner wallet
    if miner_rpc.get_balance(None, None).unwrap() == Amount::ZERO {
        for i in 1..=100 {
            miner_rpc
                .generate_to_address(1, &miner_address.clone().assume_checked())
                .unwrap();
            let balance = miner_rpc.get_balance(None, None).unwrap();
            println!("After {} blocks: Balance = {}", i * 1, balance);
        }
    }
    miner_rpc
        .generate_to_address(1, &miner_address.clone().assume_checked())
        .unwrap();
    println!(
        "Total miner wallet balance: {}",
        miner_rpc.get_balance(None, None).unwrap()
    );

    // How many blocks needs to be mined to generate a spendable balance?
    /*
     * Bitcoin block rewards (coinbase transactions) have a maturity period of 100 blocks before they become spendable.
     * This is a consensus rule designed to prevent issues if the blockchain reorganizes and coinbase transactions become invalid.
     *
     * When you mine block 1, you get a 625000000 SAT (6.25 BTC) reward, but it's "immature" and shows 0 spendable balance.
     * You need to mine 100 more blocks (blocks 2-101) before that first reward becomes mature.
     * At block 101, the reward from block 1 finally becomes spendable, showing a positive balance.
     *
     * This maturity ensures network security and prevents manipulation or double-spending of newly mined coins.
     */

    // Create receiving address from Trader wallet
    let trader_rpc = Client::new(
        format!("{RPC_URL}/wallet/{}", wallets[1]).as_str(),
        Auth::UserPass(RPC_USER.to_owned(), RPC_PASS.to_owned()),
    )?;
    let trader_address = trader_rpc.get_new_address(Some("Received"), None).unwrap();

    // Send 20 BTC from Miner to Trader
    let amount = Amount::from_int_btc(20);
    let txid = miner_rpc
        .send_to_address(
            &trader_address.clone().assume_checked(),
            amount,
            None,
            None,
            None,
            None,
            None,
            None,
        )
        .unwrap();
    println!("TxId: {:?}", txid);
    
    // Check transaction in mempool
    let mempool_data = miner_rpc.get_mempool_entry(&txid).unwrap();
    println!("Mempool Tx Data{:?}", mempool_data);
    
    // Mine 1 block to confirm the transaction
    miner_rpc
    .generate_to_address(1, &miner_address.clone().assume_checked())
    .unwrap();
    println!(
        "Total trader wallet balance: {}",
        trader_rpc.get_balance(None, None).unwrap()
    );

    // Obtain all required transaction details and get raw transaction
    let tx_details = miner_rpc.get_transaction(&txid, None).unwrap();
    let transaction_id = tx_details.info.txid;
    let raw_tx = miner_rpc.get_raw_transaction(&txid, None).unwrap();

    // Extract miner's input address and amount from the first input's previous output
    let first_input = &raw_tx.input[0];
    let prev_tx = miner_rpc.get_raw_transaction(&first_input.previous_output.txid, None).unwrap();
    let prev_output = &prev_tx.output[first_input.previous_output.vout as usize];
    let miner_input_amount = prev_output.value;

    // Get miner's input address by decoding the script of previous output
    let miner_input_address = match bitcoin::Address::from_script(&prev_output.script_pubkey, bitcoin::Network::Regtest) {
        Ok(addr) => addr.to_string(),
        Err(_) => "Unable to decode".to_string(),
    };

    // Scan for miner change details and trader output details
    let mut trader_output_amount = Amount::ZERO;
    let trader_address_str = trader_address.assume_checked().to_string();
    let mut miner_change_address = String::new();
    let mut miner_change_amount = Amount::ZERO;

    for output in &raw_tx.output {
        if let Ok(addr) = bitcoin::Address::from_script(&output.script_pubkey, Network::Regtest) {
            if addr.to_string() == trader_address_str {
                trader_output_amount = output.value;
            } else {
                miner_change_address = addr.to_string();
                miner_change_amount = output.value;
            }
        }
    }

    // Store other transaction details
    let transaction_fees = tx_details.fee.unwrap();
    let block_height = tx_details.info.blockheight.unwrap();
    let block_hash = tx_details.info.blockhash.unwrap();

    // Write the data to ../out.txt in the specified format given in readme.me
    let mut file = File::create("../out.txt")?;
    writeln!(file, "{}", transaction_id)?;
    writeln!(file, "{}", miner_input_address)?;
    writeln!(file, "{}", miner_input_amount.to_btc())?;
    writeln!(file, "{}", trader_address_str)?;
    writeln!(file, "{}", trader_output_amount.to_btc())?;
    writeln!(file, "{}", miner_change_address)?;
    writeln!(file, "{}", miner_change_amount.to_btc())?;
    writeln!(file, "{}", transaction_fees.to_btc())?;
    writeln!(file, "{:?}", block_height)?;
    writeln!(file, "{:?}", block_hash)?;

    Ok(())
}
