mod net;
mod data;
pub mod error;
mod util;
mod client;
mod config;
pub mod constants;
pub use data::model;
pub use config::*;
pub use client::*;
pub use data::ServiceChangeListener;
pub use net::{NamingRemote, HttpNamingRemote};

#[cfg(test)]
mod test {

    use std::time::Duration;

    use async_trait::async_trait;

    use crate::{
        NamingClient, config::{NamingConfig, ServerConfig}, constants, 
        error::Result, model::Instance, ServiceChangeListener
    };

    fn init_logger() {
        let _ = env_logger::builder()
            // Include all events in tests
            .filter_level(log::LevelFilter::max())
            // Ensure events are captured by `cargo test`
            .is_test(true)
            // Ignore errors initializing the logger if tests race to configure it
            .try_init();
    }

    #[tokio::test]
    
    async fn test_beat() -> Result<()> {
        init_logger();
        let config = NamingConfig {
            namespace_id: "public".to_string(),
            cluster: constants::DEFAULT_CLUSTER.to_owned(),
            group: constants::DEFAULT_GROUP.to_owned(),
            server_list: vec![ServerConfig::new(
                "http".to_string(), "192.168.1.221:8848".to_string(), "nacos".to_string()
            )],
            cache_dir: "/workspaces/nacos-sdk-rust/output/failover".to_owned(),
            load_at_start: false,
            update_when_empty: false,
            user_name: Some("nacos".to_string()),
            password: Some("nacos".to_string()),
        };
        let client = NamingClient::new_http(config).await;
        
        // client.register_instance(Instance::new_with_defaults("test", "192.168.1.221", 8888)).await?;
        client.subscribe("service-im", "DEFAULT_GROUP", vec!["DEFAULT"], Listener).await?;
        //let instances = client.select_instances("c4", "DEFAULT_GROUP", vec!["DEFAULT"], false).await?;
        //println!("server data => \n{:?}", instances);
        tokio::time::sleep(Duration::from_secs(60 * 10)).await;

        Ok(())
    }

    struct Listener;

    #[async_trait]
    impl ServiceChangeListener for Listener {
        async fn changed(&self, service_name: &str, hosts: Vec<Instance>) {
            println!("changed => {}:{:?}", service_name, hosts);
        }
    }

}

