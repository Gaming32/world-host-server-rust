use crate::server_state::ServerState;
use chrono::Local;
use log::{error, info};
use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;
use tokio::fs;
use tokio::io::AsyncWriteExt;
use tokio::time::{interval_at, Instant, MissedTickBehavior};
use try_catch::catch;

pub async fn run_analytics(server: Arc<ServerState>) {
    let analytics_time = server.config.analytics_time;
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
        let mut total = 0;
        let mut by_country = HashMap::new();
        {
            for connection in server.connections.lock().await.iter() {
                if let Some(country) = connection.state.lock().await.country {
                    by_country
                        .entry(country)
                        .and_modify(|count| *count += 1)
                        .or_insert(1);
                }
                total += 1;
            }
        }
        let country_string = by_country
            .into_iter()
            .map(|(country, count)| format!("{country}:{count}"))
            .collect::<Vec<String>>()
            .join(";");
        catch! {
            try {
                fs::OpenOptions::new()
                    .append(true)
                    .open(path)
                    .await?
                    .write_all(format!("{timestamp},{total},{country_string}\n").as_bytes())
                    .await?;
            } catch error {
                error!("Failed to write to analytics.csv: {error}");
            }
        }
    }
}
