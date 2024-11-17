use std::fmt::{Display, Formatter};
use std::time::Duration;

#[derive(Clone, Debug)]
pub struct RateLimited {
    pub bucket: String,
    pub remaining: Duration,
}

impl RateLimited {
    pub fn new(bucket: String, remaining: Duration) -> Self {
        Self { bucket, remaining }
    }
}

impl Display for RateLimited {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!(
            "Exceeded the bucket {}. Try again in {:?}.",
            self.bucket, self.remaining
        ))
    }
}
