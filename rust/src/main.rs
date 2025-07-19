#![allow(unused)]
use bitcoin::hex::DisplayHex;
use bitcoincore_rpc::bitcoin::Amount;
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

    let args = [json!({
        "outputs": { addr: 100.0 }, // Send 100 BTC to the address
        "conf_target": null,
        "estimate_mode": null,
        "fee_rate": null,
        "options": {}
    })];

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

    // Get blockchain info
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
                    match rpc.create_wallet(wallet_name, Some(false), Some(false), Some(""), Some(false)) {
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

    // Generate spendable balances in the Miner wallet. How many blocks needs to be mined?

    // Load Trader wallet and generate a new address

    // Send 20 BTC from Miner to Trader

    // Check transaction in mempool

    // Mine 1 block to confirm the transaction

    // Extract all required transaction details

    // Write the data to ../out.txt in the specified format given in readme.md

    Ok(())
}
