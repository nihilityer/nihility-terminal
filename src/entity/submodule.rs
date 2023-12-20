use std::collections::HashMap;

use nihility_common::{ClientType, ConnectionType, NihilityClient};

pub struct Submodule {
    pub name: String,
    pub default_instruct_map: HashMap<String, String>,
    pub connection_type: ConnectionType,
    pub client_type: ClientType,
    pub heartbeat_time: u64,
    pub client: Box<dyn NihilityClient + Send + Sync>,
}
