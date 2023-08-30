use nihility_common::module_info::ModuleInfoReq;

pub struct ModuleInfo {
    pub name: String,
    pub grpc_addr: String,
}

impl ModuleInfo {
    pub fn create_by_req(req: ModuleInfoReq) -> Self {
        ModuleInfo {
            name: req.name,
            grpc_addr: req.grpc_addr,
        }
    }
}