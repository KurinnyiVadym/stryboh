use azeventhubs::consumer::{
    EventHubConsumerClient, EventHubConsumerClientOptions, ReadEventOptions,
};
use dotenv::dotenv;
use fred::{
    prelude::*,
    types::{Encoding, Timestamp},
};
use futures_util::StreamExt;
use std::{collections::HashMap, env};
use telemetry::Telemetry;
mod telemetry;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut error_count = 1;
    loop {
        if let Err(e) = consumer().await {
            println!("{}", &e);
            error_count = error_count + 1;
            println!("count errors: {}", &error_count);
        }
    }
}

async fn consumer() -> Result<(), Box<dyn std::error::Error>> {
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

    let mut counter = 0;
    let mut last_device = None;
    let mut key_cache = HashMap::<String, String>::new();
    while let Some(event) = stream.next().await {
        let event = event?;
        let body = event.body()?;
        match serde_json::from_slice::<Telemetry>(body) {
            Ok(telemetry) => {
                if let None = telemetry.device_id {
                    println!("Unknown device send message")
                }
                if let None = telemetry.time_stamp {
                    println!(
                        "Device {:?} sends message without TimeStamp",
                        telemetry.device_id
                    )
                }
                let device_id = telemetry.device_id.unwrap_or(String::from("Unknown"));
                last_device = Some(device_id.clone());
                let t_key = ensure_key(&client, &mut key_cache, &device_id, "temperature").await?;
                let h_key = ensure_key(&client, &mut key_cache, &device_id, "humidity").await?;

                let ts: Timestamp =
                    Timestamp::from(telemetry.time_stamp.unwrap_or_default() * 1000);

                let mut value_pairs: Vec<(RedisKey, Timestamp, f64)> = Vec::new();
                value_pairs.push((RedisKey::try_from(t_key)?, ts.clone(), telemetry.temperature));
                value_pairs.push((RedisKey::try_from(h_key)?, ts.clone(), telemetry.humidity));

                if let Some(pressure) =telemetry.pressure{
                    let p_key = ensure_key(&client, &mut key_cache, &device_id, "pressure").await?;
                    value_pairs.push((RedisKey::try_from(p_key)?, ts.clone(), pressure));
                }

                client.ts_madd(value_pairs).await?;
            }
            Err(e) => {
                println!("{e}");
                let value = std::str::from_utf8(body)?;
                println!("{}", value);
            }
        }

        println!("counter: {} {:?}", counter, last_device);
        counter += 1;
    }
    // Close the stream
    stream.close().await?;

    // Close the consumer client
    consumer_client.close().await?;

    Ok(())
}

async fn ensure_key(
    client: &RedisClient,
    map: &mut HashMap<String, String>,
    device_id: &str,
    telemetry: &str,
) -> Result<String, Box<dyn std::error::Error>> {
    let key = format!("{}:{}", telemetry, device_id);

    if map.contains_key(&key) {
        return Ok(key);
    }
    if client.exists(&key).await? {
        map.insert(key.clone(), String::from(""));
        return Ok(key);
    }
    let retention = Some(365 * 24 * 60 * 60 * 1000);
    let encoding = Some(Encoding::Compressed);
    let chunk_size = None;
    let duplicate_policy = Some(fred::types::DuplicatePolicy::Last);
    let labels = HashMap::from([("id", device_id), ("telemetry", telemetry)]);
    println!("creating a new timeseries {}", &key);

    client
        .ts_create(
            &key,
            retention,
            encoding,
            chunk_size,
            duplicate_policy,
            labels,
        )
        .await?;
    println!("{} has been created", &key);
    map.insert(key.clone(), String::from(""));

    return Ok(key);
}
