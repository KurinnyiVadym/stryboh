use azeventhubs::consumer::{
    EventHubConsumerClient, EventHubConsumerClientOptions, ReadEventOptions,
};
use dotenv::dotenv;
use fred::{prelude::*, types::Timestamp};
use futures_util::StreamExt;
use std::env;
use telemetry::Telemetry;
mod telemetry;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenv().ok();

    let mut consumer_client = EventHubConsumerClient::new_from_connection_string(
        EventHubConsumerClient::DEFAULT_CONSUMER_GROUP_NAME, //"viter-consumer".to_string(),
        env::var("EVENT_HUB_CONNNECTION_STRING")
            .expect("EVENT_HUB_CONNNECTION_STRING not specifyed"), // Replace with your connection string
        env::var("EVENT_HUB_NAME").expect("EVENT_HUB_NAME not specifyed"),
        EventHubConsumerClientOptions::default(),
    )
    .await?;

    let options = ReadEventOptions::default();

    // Get a stream of events from the first partition
    let mut stream = consumer_client
        .read_events(true, options) //
        .await?;

    let config = RedisConfig::from_url(&env::var("REDIS_URL").expect("REDIS_URL not specifyed"))?;
    
    let client = Builder::from_config(config).build()?; 
    client.init().await?;
    // client.ts_create(key, retention, encoding, chunk_size, duplicate_policy, labels) // todo!!!
    
    // Receive 30 events
    let mut counter = 0;
    while let Some(event) = stream.next().await {
        let event = event?;
        let body = event.body()?;
        match serde_json::from_slice::<Telemetry>(body) {
            Ok(telemetry) => {
                if let None = telemetry.device_id {
                    print!("Unknown device send message")
                }
                if let None = telemetry.time_stamp {
                    print!(
                        "Device {:?} sends message without TimeStamp",
                        telemetry.device_id
                    )
                }
                let t_key = format!("temperature:{}", telemetry.device_id.clone().unwrap_or_default());
                let h_key = format!("humidity:{}", telemetry.device_id.unwrap_or_default());
                
                let ts: Timestamp = Timestamp::from(telemetry.time_stamp.unwrap_or_default() * 1000);
                let samples: [(RedisKey, Timestamp, f64); 2] = [
                    (RedisKey::try_from(t_key)?,
                    ts, telemetry.temperature), 
                    (RedisKey::try_from(h_key)?,
                    Timestamp::from(telemetry.time_stamp.unwrap_or_default() * 1000), telemetry.humidity), ];
                client.ts_madd(samples).await?;
            }
            Err(e) => {
                println!("{e}");
                let value = std::str::from_utf8(body)?;
                println!("{}", value);
            }
        }
        // let value = std::str::from_utf8(body)?;

        println!("counter: {}", counter);
        counter += 1;
    }
    // Close the stream
    stream.close().await?;

    // Close the consumer client
    consumer_client.close().await?;

    Ok(())
}
