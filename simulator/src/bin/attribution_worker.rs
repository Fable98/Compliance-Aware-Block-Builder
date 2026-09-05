use alloy::providers::{Provider, ProviderBuilder};
use sqlx::postgres::PgPoolOptions;
use std::time::Duration;

const ANVIL_RPC: &str = "http://127.0.0.1:8545";
const DATABASE_URL: &str = "postgres://shresthkumar@localhost:5432/compliance_builder";

#[tokio::main]
async fn main() -> eyre::Result<()> {
    let provider = ProviderBuilder::new().connect_http(ANVIL_RPC.parse()?);
    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(DATABASE_URL)
        .await?;

    println!("Attribution worker started. Polling for new blocks...");

    let mut last_seen_block: u64 = 0;

    loop {
        let latest_block_number = provider.get_block_number().await?;

        if latest_block_number > last_seen_block {
            for block_num in (last_seen_block + 1)..=latest_block_number {
                if let Some(block) = provider.get_block_by_number(block_num.into()).await? {
                    let miner = format!("{:?}", block.header.beneficiary).to_lowercase();
                    let block_hash = format!("{:?}", block.header.hash);
                    let tx_count = block.transactions.len() as i32;

                    // Check if this address is a known sanctioned entity
                    let attribution: Option<(String, uuid::Uuid)> = sqlx::query_as(
                        "SELECT attribution_type, entity_id FROM address_attributions WHERE address = $1",
                    )
                    .bind(&miner)
                    .fetch_optional(&pool)
                    .await?;

                    let (compliance_status, proposer_entity_id) = match &attribution {
                        Some((_, entity_id)) => {
                            println!(
                                "[Block {}] ALERT: proposer {} is a SANCTIONED entity ({})",
                                block_num, miner, entity_id
                            );
                            ("EXPOSED_EXTERNAL", Some(*entity_id))
                        }
                        None => {
                            println!(
                                "[Block {}] proposer {} — no match, status UNATTRIBUTED",
                                block_num, miner
                            );
                            ("COMPLIANT_BUILD", None)
                        }
                    };

                    sqlx::query(
                        "INSERT INTO blocks (block_hash, block_number, builder_address, proposer_entity_id, tx_count, compliance_status)
                         VALUES ($1, $2, $3, $4, $5, $6)
                         ON CONFLICT (block_hash) DO NOTHING",
                    )
                    .bind(&block_hash)
                    .bind(block_num as i64)
                    .bind(&miner)
                    .bind(proposer_entity_id)
                    .bind(tx_count)
                    .bind(compliance_status)
                    .execute(&pool)
                    .await?;
                }
            }
            last_seen_block = latest_block_number;
        }

        tokio::time::sleep(Duration::from_secs(2)).await;
    }
}
