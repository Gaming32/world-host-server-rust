use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Copy, Clone)]
pub struct LatitudeLongitude(pub f64, pub f64);

impl LatitudeLongitude {
    pub fn haversine_distance(&self, other: &LatitudeLongitude) -> f64 {
        let x1 = self.0.to_radians();
        let y1 = self.1.to_radians();
        let x2 = other.0.to_radians();
        let y2 = other.1.to_radians();

        let a =
            ((x2 - x1) / 2.0).sin().powi(2) + x1.cos() * x2.cos() * ((y2 - y1) / 2.0).sin().powi(2);

        2.0 * f64::min(1.0, a.sqrt()).asin()
    }
}
