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
#[allow(dead_code)]
struct ScreenResponse {
    tx_hash: String,
    decision: String,
    risk_score: i32,
    reasons: Vec<String>,
}

async fn screen_transaction(
    client: &reqwest::Client,
    engine_url: &str,
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
        .post(engine_url)
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

// Anvil default account #1 private key (clean sender with zero exposure)
const CLEAN_SENDER_PRIVATE_KEY: &str = "59c6995e998f97a5a0044966f0945389dc9e86dae88c7a8412f4603b6b78690d";

// Anvil default account #0 private key (sender used for sanctions & indirect exposure scenarios)
const SENDER_PRIVATE_KEY: &str = "ac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80";

// A real OFAC-sanctioned address from our seeded list
const SANCTIONED_ADDRESS: &str = "0x0330070FD38Ec3bB94F58FA55D40368271E9e54A";

// Anvil default account #3 (clean recipient)
const CLEAN_RECIPIENT: &str = "0x90F79bf6EB2c4f870365E785982E1f101E93b906";

#[tokio::main]
async fn main() -> eyre::Result<()> {
    dotenvy::dotenv().ok();
    let anvil_rpc = std::env::var("ANVIL_RPC")
        .unwrap_or_else(|_| "http://127.0.0.1:8545".to_string());
    let engine_url = std::env::var("ENGINE_URL")
        .unwrap_or_else(|_| "http://127.0.0.1:3001/screen".to_string());

    let http_client = reqwest::Client::new();

    let clean_signer = PrivateKeySigner::from_str(CLEAN_SENDER_PRIVATE_KEY)?;
    let clean_sender_address = clean_signer.address();
    let clean_wallet = EthereumWallet::from(clean_signer);
    let clean_provider = ProviderBuilder::new()
        .wallet(clean_wallet)
        .connect_http(anvil_rpc.parse()?);

    let signer = PrivateKeySigner::from_str(SENDER_PRIVATE_KEY)?;
    let sender_address = signer.address();
    let wallet = EthereumWallet::from(signer);
    let provider = ProviderBuilder::new()
        .wallet(wallet)
        .connect_http(anvil_rpc.parse()?);

    println!("=== Scenario 1: Clean transaction ===");
    let recipient = Address::from_str(CLEAN_RECIPIENT)?;
    let fake_tx_hash = "0xsim001";

    let decision = screen_transaction(
        &http_client,
        &engine_url,
        fake_tx_hash,
        &format!("{:?}", clean_sender_address),
        &format!("{:?}", recipient),
    )
    .await?;

    println!("Compliance decision: {:?}", decision);

    if decision.decision == "ALLOW" {
        submit_transaction(&clean_provider, clean_sender_address, recipient, "Scenario 1").await?;
    } else {
        println!("[Scenario 1] BLOCKED before submission to chain.");
    }

    // Pause for pacing & live dashboard telemetry
    tokio::time::sleep(std::time::Duration::from_millis(1500)).await;

    println!("\n=== Scenario 2: Sanctioned recipient ===");
    let sanctioned = Address::from_str(SANCTIONED_ADDRESS)?;
    let fake_tx_hash_2 = "0xsim002";

    let decision2 = screen_transaction(
        &http_client,
        &engine_url,
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

    // Guarantee Scenario 2 DB row is persisted and give dashboard/AI-explainer time to narrate
    tokio::time::sleep(std::time::Duration::from_millis(1500)).await;

    println!("\n=== Scenario 3: Indirect exposure (1-hop counterparty to sanctioned entity) ===");
    let clean_recipient = Address::from_str(CLEAN_RECIPIENT)?;
    let fake_tx_hash_3 = "0xsim003";

    let decision3 = screen_transaction(
        &http_client,
        &engine_url,
        fake_tx_hash_3,
        &format!("{:?}", sender_address),
        &format!("{:?}", clean_recipient),
    )
    .await?;

    println!("Compliance decision: {:?}", decision3);

    if decision3.decision == "ALLOW" {
        submit_transaction(&provider, sender_address, clean_recipient, "Scenario 3").await?;
    } else if decision3.decision == "FLAG" {
        println!(
            "[Scenario 3] FLAGGED for human review / enhanced due diligence (risk score: {}) — 1-hop graph walk detected indirect exposure to sanctioned entity.",
            decision3.risk_score
        );
    } else {
        println!("[Scenario 3] BLOCKED before submission to chain.");
    }

    Ok(())
}
