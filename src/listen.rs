use crate::influx::{Influx, LiquidationTransaction};
use chrono::Utc;
use futures::StreamExt;
use solana_sdk::{inflation, signature::Signature, transaction};
use solana_transaction_status::TransactionWithStatusMeta;
use std::collections::HashMap;
use yellowstone_grpc_client::GeyserGrpcClient;
use yellowstone_grpc_proto::{
    convert_from,
    geyser::subscribe_update::UpdateOneof,
    prelude::{
        CommitmentLevel, SubscribeRequest, SubscribeRequestFilterTransactions,
        SubscribeUpdateTransaction,
    },
};
pub struct Listener;

impl Listener {
    pub async fn start(endpoint: String, x_token: String, influx: Influx) -> anyhow::Result<()> {
        loop {
            let mut geyser_client = match GeyserGrpcClient::build_from_shared(endpoint.clone())?
                .x_token(Some(x_token.clone()))
            {
                Ok(builder) => match builder.connect().await {
                    Ok(geyser_client_connection) => {
                        println!("Successfully connected to Geyser service");
                        geyser_client_connection
                    }
                    Err(e) => {
                        eprintln!("Failed to connect: {}", e);
                        continue;
                    }
                },
                Err(e) => {
                    eprintln!("Failed to build client: {}", e);
                    continue;
                }
            };
            let subscribe_request = SubscribeRequest {
                transactions: HashMap::from_iter([(
                    "MFv2hWf31Z9kbCa1snEPYctwafyhdvnV7FZnsebVacA".to_string(),
                    SubscribeRequestFilterTransactions {
                        vote: Some(false),
                        failed: Some(false),
                        account_include: vec![
                            "MFv2hWf31Z9kbCa1snEPYctwafyhdvnV7FZnsebVacA".to_string()
                        ],
                        account_exclude: vec![],
                        ..Default::default()
                    },
                )]),
                commitment: Some(CommitmentLevel::Confirmed as i32),
                ..Default::default()
            };

            let (_, mut stream) = match geyser_client
                .subscribe_with_request(Some(subscribe_request))
                .await
            {
                Ok(value) => value,
                Err(e) => {
                    eprintln!("Error creating a stream: {:?}", e);
                    continue;
                }
            };

            while let Some(received) = stream.next().await {
                match received {
                    Ok(received) => {
                        if let Some(update) = received.update_oneof {
                            if let Ok(Some(liquidation)) = Self::process(update) {
                                let rs = influx.push_liquidation(liquidation).await;
                                if let Err(e) = rs {
                                    println!("Error writing to influx: {:?}", e);
                                }
                            }
                        }
                    }
                    Err(err) => {
                        println!("Ërror pooling next update: {:?}", err);
                        break;
                    }
                }
            }
        }
    }

    fn process(update: UpdateOneof) -> anyhow::Result<Option<LiquidationTransaction>> {
        match update {
            UpdateOneof::Transaction(transaction_update) => {
                if let Some(transaction_info) = transaction_update.transaction {
                    let signature = Signature::try_from(transaction_info.signature.clone())
                        .unwrap()
                        .to_string();
                    let transaction = convert_from::create_tx_with_meta(transaction_info).unwrap();
                    let transaction = match transaction {
                        TransactionWithStatusMeta::Complete(transaction) => transaction,
                        _ => return Ok(None),
                    };
                    let signer = transaction
                        .transaction
                        .message
                        .static_account_keys()
                        .first()
                        .unwrap()
                        .to_string();
                    /* 
                    let is_not_liquidation = !transaction
                        .meta
                        .log_messages
                        .unwrap_or_default()
                        .into_iter()
                        .map(|log| {
                            log.contains(
                                &"Program log: Instruction: LendingAccountLiquidate".to_string(),
                            )
                        })
                        .collect::<Vec<_>>()
                        .is_empty();
                    */
                    let messages = transaction.meta.log_messages.unwrap_or_default();
                    for message in messages {
                        if message.contains("LendingAccountLiquidate") {
                            return Ok(Some(LiquidationTransaction {
                                time: Utc::now().timestamp_nanos() as u64,
                                signer,
                                signature,
                            }));
                        }
                    }
                    /* 
                    if !is_not_liquidation {
                        return Ok(Some(LiquidationTransaction {
                            time: Utc::now().timestamp_nanos() as u64,
                            signer,
                            signature,
                        }));
                    };
                    */
                }
                return Ok(None);
            }
            _ => {}
        }
        Ok(None)
    }
}
