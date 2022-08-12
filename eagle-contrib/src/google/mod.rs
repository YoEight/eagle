mod generated;
mod sinks;

mod api {
    pub use crate::google::generated::google_api::*;
}

mod rpc {
    pub use crate::google::generated::google_rpc::*;
}
