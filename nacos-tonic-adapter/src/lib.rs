use std::sync::Arc;

use crossbeam::queue::SegQueue;
use nacos_naming_client:: {
    NamingClient, HttpNamingRemote, NamingConfig, constants, ServerConfig,
    ServiceChangeListener, model::Instance,
    error::Result
};

use async_once::AsyncOnce;
use lazy_static::lazy_static;
use async_trait::async_trait;
use tower::discover::Change;
use tonic::transport::{Endpoint, Channel};
use log;

pub use nacos_naming_client::error;

lazy_static! {
    static ref NACOS_CLIENT: AsyncOnce<NamingClient<HttpNamingRemote>> = AsyncOnce::new(async {
        gen_client_from_env().await
    });
}


async fn gen_client_from_env() -> NamingClient<HttpNamingRemote> {
    use std::env::var;
    let namespace = var("NACOS_NAMESPACE").unwrap_or(constants::DEFAULT_NAMSPACE.to_owned());
    let group = var("NACOS_GROUP").unwrap_or(constants::DEFAULT_GROUP.to_owned());
    let cluster = var("NACOS_CLUSTER").unwrap_or(constants::DEFAULT_CLUSTER.to_owned());
    let server_schema = var("NACOS_SERVER_SCHEMA").unwrap_or(constants::DEFAULT_SERVER_SCHEMA.to_owned());
    let server_context = var("NACOS_SERVER_CONTEXT").unwrap_or(constants::DEFAULT_SERVER_CONTEXT.to_owned());
     let servers = var("NACOS_SERVER_ADDRS").expect("env NACOS_SERVER_ADDRS is empty")
        .split(",")
        .map(|server| ServerConfig::new(server_schema.clone(), server.to_string(), server_context.clone()))
        .collect::<Vec<_>>();
    let failover_dir =var("NACOS_NAMING_FAILOVER").unwrap_or_else(|_|{
        let home = var("HOME").expect("can't obtain home dir");
        format!("{}/{}", home, constants::DEFAULT_FAILOVER_DIR)
    });

    let config = NamingConfig {
        namespace_id: namespace,
        cluster,
        group,
        server_list: servers,
        cache_dir: failover_dir,
        load_at_start: parse_bool_env("NACOS_NAMING_LOAD_AT_START", false),
        update_when_empty: parse_bool_env("NACOS_NAMING_LOAD_AT_START", false),
        user_name: var("NACOS_USERNAME").ok(),
        password: var("NACOS_PASSWORD").ok(),
    };
    NamingClient::new_http(config).await
}

fn parse_bool_env(key: &str, default: bool) -> bool {
    std::env::var(key).map(|res|{
        match res.parse::<bool>() {
            Ok(x) => x,
            Err(error) => {
                log::warn!("failed to parse nacos env[{}]: {}", key, error);
                default
            }
        }
    }).unwrap_or(default)
}
struct ChangeListener {
    rx: tokio::sync::mpsc::Sender<Change<String, Endpoint>>,
    queue: Arc<SegQueue<Change<String, Endpoint>>>,
}

#[async_trait]
impl ServiceChangeListener for ChangeListener {
    async fn changed(&self, service_name: &str, hosts: Vec<Instance>) {
        log::debug!("obtain service[{}] change from nacos: {:?}", service_name, hosts);
        for mut instance in hosts {
            let port = instance.metadata.remove("gRPC_port").unwrap_or(instance.port.to_string());
            let endpoint = format!("{}://{}:{}", "http", instance.ip, port);
            let ep = Endpoint::from_shared(endpoint.clone());
            let ep = match ep {
                Ok(endpoint) => endpoint,
                Err(error) => {
                    log::error!("invalid url: {}", error);
                    continue
                } 
            };
            let change = if instance.healthy && instance.enabled && instance.weight > 0f64 {
                Change::Insert(endpoint, ep)
            } else {
                Change::Remove(endpoint)
            };

            self.queue.push(change);
            if self.queue.len() > 10 {
                self.queue.pop();
            }
        }
    }
}



pub async fn gen_channel(service_name: &str) -> Channel {
    let nacos_client = NACOS_CLIENT.get().await;
    let (channel, rx) = Channel::balance_channel(1);
    let queue = Arc::new(SegQueue::new());
    let listener = ChangeListener {rx: rx.clone(), queue: queue.clone()};
    tokio::spawn(async move {
        loop {
            if let Some(change) = queue.pop() {
                if let Err(error) = rx.try_send(change) {
                    log::debug!("failed to modify endpoint list: {}", error)
                }
            }
        }
    });
    let x = nacos_client.subscribe(
        service_name, nacos_client.get_group(), vec![nacos_client.get_cluster()], listener
    ).await;
    if let Err(error) = x {
        log::error!("failed to subscribe service change: {}", error);
        return channel;
    }
    channel
}

pub async fn register(service_name: &str, group: &str, cluster: &str, ip: &str, port: u16) -> Result<()> {
    let nacos_client = NACOS_CLIENT.get().await;
    let mut instance = Instance::new_with_required(service_name, group, cluster, ip, port);
    instance.metadata.insert("gRPC_port".to_owned(), port.to_string());
    nacos_client.register_instance(instance).await
}