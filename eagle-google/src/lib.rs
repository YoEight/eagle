#[macro_use]
extern crate metrics;

mod generated;
pub mod sinks;
mod types;

pub use types::{Resource, StackDriverMetricsOptions};

mod api {
    pub use crate::generated::google_api::*;
}

mod rpc {
    pub use crate::generated::google_rpc::*;
}

use chrono::{DateTime, Utc};
use prost_types::Timestamp;

pub(crate) fn to_timestamp(time: DateTime<Utc>) -> Timestamp {
    Timestamp {
        seconds: time.timestamp(),
        nanos: time.timestamp_nanos() as i32,
    }
}
