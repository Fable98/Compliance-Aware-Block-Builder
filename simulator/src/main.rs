use alloy::network::{EthereumWallet, TransactionBuilder};
use alloy::primitives::{Address, U256};
use alloy::providers::{Provider, ProviderBuilder};
use alloy::rpc::types::TransactionRequest;
use alloy::signers::local::PrivateKeySigner;
use serde::{Deserialize, Serialize};
use std::str::FromStr;

#[derive(Serialize)]
struct ScreenRequest {
    tx_hash: String,
    sender: String,
    recipient: String,
}

#[derive(Deserialize, Debug)]
struct ScreenResponse {
    tx_hash: String,
    decision: String,
    risk_score: i32,
    reasons: Vec<String>,
}

const ANVIL_RPC: &str = "http://127.0.0.1:8545";
const ENGINE_URL: &str = "http://127.0.0.1:3001/screen";

// Anvil default account #0 private key (test funds only, never use in production)
const SENDER_PRIVATE_KEY: &str = "ac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80";

// A real OFAC-sanctioned address from our seeded list
const SANCTIONED_ADDRESS: &str = "0x0330070FD38Ec3bB94F58FA55D40368271E9e54A";

// Anvil default account #1 (clean recipient)
const CLEAN_RECIPIENT: &str = "0x70997970C51812dc3A010C7d01b50e0d17dc79C8";

async fn screen_transaction(
    client: &reqwest::Client,
    tx_hash: &str,
    sender: &str,
    recipient: &str,
) -> eyre::Result<ScreenResponse> {
    let req = ScreenRequest {
        tx_hash: tx_hash.to_string(),
        sender: sender.to_string(),
        recipient: recipient.to_string(),
    };

    let resp = client
        .post(ENGINE_URL)
        .json(&req)
        .send()
        .await?
        .json::<ScreenResponse>()
        .await?;

    Ok(resp)
}

async fn submit_transaction(
    provider: &impl Provider,
    from: Address,
    to: Address,
    label: &str,
) -> eyre::Result<()> {
    let tx = TransactionRequest::default()
        .with_from(from)
        .with_to(to)
        .with_value(U256::from(1_000_000_000_000_000_000u64)); // 1 ETH

    match provider.send_transaction(tx).await {
        Ok(pending) => {
            let receipt = pending.get_receipt().await?;
            println!(
                "[{}] Transaction included on-chain. Hash: {:?}, Status: {}",
                label,
                receipt.transaction_hash,
                receipt.status()
            );
        }
        Err(e) => {
            println!("[{}] Transaction rejected/failed: {:?}", label, e);
        }
    }

    Ok(())
}

#[tokio::main]
async fn main() -> eyre::Result<()> {
    let http_client = reqwest::Client::new();

    let signer = PrivateKeySigner::from_str(SENDER_PRIVATE_KEY)?;
    let sender_address = signer.address();
    let wallet = EthereumWallet::from(signer);

    let provider = ProviderBuilder::new()
        .wallet(wallet)
        .connect_http(ANVIL_RPC.parse()?);

    println!("=== Scenario 1: Clean transaction ===");
    let recipient = Address::from_str(CLEAN_RECIPIENT)?;
    let fake_tx_hash = "0xsim001";

    let decision = screen_transaction(
        &http_client,
        fake_tx_hash,
        &format!("{:?}", sender_address),
        &format!("{:?}", recipient),
    )
    .await?;

    println!("Compliance decision: {:?}", decision);

    if decision.decision == "ALLOW" {
        submit_transaction(&provider, sender_address, recipient, "Scenario 1").await?;
    } else {
        println!("[Scenario 1] BLOCKED before submission to chain.");
    }

    println!("\n=== Scenario 2: Sanctioned recipient ===");
    let sanctioned = Address::from_str(SANCTIONED_ADDRESS)?;
    let fake_tx_hash_2 = "0xsim002";

    let decision2 = screen_transaction(
        &http_client,
        fake_tx_hash_2,
        &format!("{:?}", sender_address),
        &format!("{:?}", sanctioned),
    )
    .await?;

    println!("Compliance decision: {:?}", decision2);

    if decision2.decision == "ALLOW" {
        submit_transaction(&provider, sender_address, sanctioned, "Scenario 2").await?;
    } else {
        println!("[Scenario 2] BLOCKED before submission to chain — compliance engine caught it.");
    }

    Ok(())
}
