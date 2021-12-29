pub struct ServerConfig {
    scheme: String,
    ip: String,
    port: u16,
    context_path: String
}

pub struct NamingConfig {
    pub namespace_id: String,
    pub cluster: String,
    pub group: String,
    pub server_list: Vec<ServerConfig>,
    pub cache_dir: String,
    pub load_at_start: bool,
    pub update_when_empty: bool
}

impl ServerConfig {
    pub fn to_string(&self) -> String {
        format!("{}://{}:{}/{}", self.scheme, self.ip, self.port, self.context_path)
    }

    pub fn new(scheme: String, ip: String, port: u16, context_path: String) -> Self {
        ServerConfig {
            scheme, 
            ip,
            port, 
            context_path
        }
    }
}