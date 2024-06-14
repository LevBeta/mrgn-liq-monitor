/// Listens to mrgn transactions
pub mod listen;

/// Influx db
pub mod influx;

#[tokio::main]
async fn main() {
    let influx_client = influx::Influx::new(
        "http://localhost:8086".to_string(),
        "".to_string(),
        ""
            .to_string(),
        "".to_string(),
    );
    let _ = listen::Listener::start(
        "".to_string(),
        "".to_string(),
        influx_client,
    )
    .await;
}
