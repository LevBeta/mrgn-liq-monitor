use futures::stream;
use influxdb2::Client;
use influxdb2_derive::WriteDataPoint;
use solana_sdk::{nonce::state::Data, pubkey::Pubkey};

pub struct Influx {
    client: Client,
    bucket: String,
}

impl Influx {
    pub fn new(url: String, org: String, token: String, bucket: String) -> Self {
        Influx {
            client: Client::new(url, org, token),
            bucket,
        }
    }

    pub async fn push_liquidation(
        &self,
        liquidation_trxn: LiquidationTransaction,
    ) -> anyhow::Result<()> {
        self.client
            .write(&self.bucket, stream::iter(vec![liquidation_trxn]))
            .await?;
        Ok(())
    }
}

#[derive(WriteDataPoint, Debug, Default)]
#[measurement = "liquidations"]
pub struct LiquidationTransaction {
    #[influxdb(timestamp)]
    pub time: u64,
    #[influxdb(tag)]
    pub signer: String,
    #[influxdb(field)]
    pub signature: String,
}
