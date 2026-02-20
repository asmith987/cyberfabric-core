use authz_resolver_sdk::pep::ResourceType;

pub(crate) mod headers;
pub(crate) mod request_builder;
pub(crate) mod service;

pub(crate) use service::DataPlaneServiceImpl;

pub(crate) mod resources {
    use super::ResourceType;

    pub const PROXY: ResourceType = ResourceType {
        name: "gts.x.core.oagw.proxy.v1~",
        supported_properties: &[],
    };
}

pub(crate) mod actions {
    pub const INVOKE: &str = "invoke";
}