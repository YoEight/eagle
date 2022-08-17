mod generated;
mod sinks;

mod api {
    pub use crate::google::generated::google_api::*;
}

mod rpc {
    pub use crate::google::generated::google_rpc::*;
}

use chrono::{DateTime, Utc};
use prost_types::Timestamp;

pub(crate) fn to_timestamp(time: DateTime<Utc>) -> Timestamp {
    Timestamp {
        seconds: time.timestamp(),
        nanos: time.timestamp_nanos() as i32,
    }
}
