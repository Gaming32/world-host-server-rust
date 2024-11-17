use crate::authlib::auth_service::YggdrasilAuthenticationService;
use crate::server_state::ServerState;
use crate::util::ip_info_map::IpInfoMap;
use log::{error, info};
use std::time::Instant;

pub async fn run_main_server(server: &ServerState) {
    let session_service = YggdrasilAuthenticationService::new().create_session_service();
    let ip_info_map = load_ip_info_map().await;
}

async fn load_ip_info_map() -> IpInfoMap {
    info!("Downloading IP info map...");
    let start = Instant::now();
    let result = IpInfoMap::load_from_compressed_geolite_city_files(
        if !cfg!(debug_assertions) { // This takes a whopping 15 seconds (on my computer) under the dev target!
            vec![
                "https://github.com/sapics/ip-location-db/raw/main/geolite2-city/geolite2-city-ipv4-num.csv.gz",
                "https://github.com/sapics/ip-location-db/raw/main/geolite2-city/geolite2-city-ipv6-num.csv.gz",
            ]
        } else {
            vec![]
        }
    ).await;
    let duration = start.elapsed();
    match result {
        Ok(map) => {
            info!(
                "Downloaded IP info map in {duration:?} ({} entries)",
                map.len()
            );
            map
        }
        Err(err) => {
            error!("Failed to download IP info map in {duration:?}: {err}");
            IpInfoMap::default()
        }
    }
}
