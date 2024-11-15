use chrono::{DateTime, Local};
use log::{error, info};
use std::collections::HashMap;
use std::path::Path;
use std::time::Duration;
use tokio::fs;
use tokio::io::AsyncWriteExt;
use tokio::time::{interval_at, Instant, MissedTickBehavior};
use try_catch::catch;

pub async fn run_analytics(analytics_time: Duration) {
    if analytics_time.is_zero() {
        return info!("Analytics disabled by request");
    }
    info!("Starting analytics system to update every {analytics_time:?}");
    let path = Path::new("analytics.csv");
    let mut interval = interval_at(Instant::now() + analytics_time, analytics_time);
    interval.set_missed_tick_behavior(MissedTickBehavior::Skip);
    loop {
        interval.tick().await;
        catch! {
            try {
                if !fs::try_exists(path).await? || fs::metadata(path).await?.len() == 0 {
                    info!("Creating new analytics.csv");
                    fs::write(path, "timestamp,total,countries\n").await?;
                }
            } catch error {
                error!("Failed to create analytics.csv: {error}");
            }
        }
        info!("Updating analytics.csv");
        let timestamp = Local::now().format("%+");
        #[allow(unused_mut)] let mut total = 0;
        let by_country: HashMap<String, u32> = HashMap::new();
        // TODO: Implement analytics and remove above explicit args
        let country_string = by_country.into_iter()
            .map(|(country, count)| format!("{country}:{count}"))
            .collect::<Vec<String>>()
            .join(";");
        catch! {
            try {
                fs::OpenOptions::new()
                    .append(true)
                    .open(path)
                    .await?
                    .write(format!("{timestamp},{total},{country_string}\n").as_bytes())
                    .await?;
            } catch error {
                error!("Failed to write to analytics.csv: {error}");
            }
        }
    }
}
