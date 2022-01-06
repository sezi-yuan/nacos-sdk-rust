pub struct ServerConfig {
    scheme: String,
    address: String,
    context_path: String
}

pub struct NamingConfig {
    pub namespace_id: String,
    pub cluster: String,
    pub group: String,
    pub server_list: Vec<ServerConfig>,
    pub cache_dir: String,
    pub load_at_start: bool,
    pub update_when_empty: bool,
    pub user_name: Option<String>,
    pub password: Option<String>
}

impl ServerConfig {
    pub fn to_string(&self) -> String {
        format!("{}://{}/{}", self.scheme, self.address, self.context_path)
    }

    pub fn new(scheme: String, address: String, context_path: String) -> Self {
        ServerConfig {
            scheme, 
            address,
            context_path
        }
    }
}