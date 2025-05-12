use crate::lat_long::LatitudeLongitude;
use crate::util::ip_info::IpInfo;
use crate::util::range_map::{U128ToU32RangeMap, U32ToU32RangeMap};
use async_compression::tokio::bufread::GzipDecoder;
use futures::{StreamExt, TryStreamExt};
use log::error;
use reqwest::IntoUrl;
use std::net::IpAddr;
use tokio_util::compat::TokioAsyncReadCompatExt;
use tokio_util::io::StreamReader;

pub struct IpInfoMap {
    four_map: U32ToU32RangeMap,
    six_map: U128ToU32RangeMap,
}

const U32_MAX: u128 = u32::MAX as u128;

impl IpInfoMap {
    pub async fn load_from_compressed_geolite_city_files<T: IntoUrl>(
        urls: Vec<T>,
    ) -> anyhow::Result<Self> {
        let mut four_map = U32ToU32RangeMap::new();
        let mut six_map = U128ToU32RangeMap::new();
        for url in urls {
            csv_async::AsyncReader::from_reader(
                GzipDecoder::new(StreamReader::new(
                    reqwest::get(url)
                        .await?
                        .bytes_stream()
                        .map_err(std::io::Error::other),
                ))
                .compat(),
            )
            .into_records()
            .for_each(|record| {
                match parse_record(record) {
                    Ok(info) => {
                        if let Some((start_of_range, end_of_range, info)) = info {
                            if end_of_range < U32_MAX {
                                four_map.put(start_of_range as u32, end_of_range as u32, info);
                            } else {
                                six_map.put(start_of_range, end_of_range, info);
                            }
                        }
                    }
                    Err(err) => error!("Failed to parse record: {err:?}"),
                }
                futures::future::ready(())
            })
            .await;
        }
        four_map.shrink_to_fit();
        six_map.shrink_to_fit();
        Ok(Self { four_map, six_map })
    }

    pub fn get(&self, addr: IpAddr) -> Option<IpInfo> {
        let addr_bits = match addr {
            IpAddr::V4(ipv4) => ipv4.to_bits() as u128,
            IpAddr::V6(ipv6) => ipv6.to_bits(),
        };
        if addr_bits <= U32_MAX {
            self.four_map.get(&(addr_bits as u32))
        } else {
            self.six_map.get(&addr_bits)
        }
        .map(IpInfo::from_u32)
    }

    pub fn len(&self) -> usize {
        self.four_map.len() + self.six_map.len()
    }
}

fn parse_record(
    record: csv_async::Result<csv_async::StringRecord>,
) -> anyhow::Result<Option<(u128, u128, u32)>> {
    let record = record?;
    if record.len() < 9 || record[7].is_empty() || record[8].is_empty() {
        return Ok(None);
    }
    let start_of_range = record[0].parse()?;
    let end_of_range = record[1].parse()?;
    let country = record[2].parse()?;
    let lat = record[7].parse()?;
    let long = record[8].parse()?;
    let ip_info = IpInfo {
        country,
        lat_long: LatitudeLongitude(lat, long),
    };
    Ok(Some((start_of_range, end_of_range, ip_info.to_u32())))
}

impl Default for IpInfoMap {
    fn default() -> Self {
        Self {
            four_map: U32ToU32RangeMap::new(),
            six_map: U128ToU32RangeMap::new(),
        }
    }
}
