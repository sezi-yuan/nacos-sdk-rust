mod net;
mod data;
mod error;
mod util;
mod client;
mod config;
pub mod constants;

pub use config::NamingConfig;
pub use client::*;
pub use net::HttpNamingRemote;

#[cfg(test)]
mod test {

    use std::time::Duration;

    use crate::{
        NamingClient, config::{NamingConfig, ServerConfig}, constants, 
        data::model::Instance,
        error::Result
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
                "http".to_string(), "192.168.1.248".to_string(), 8848, "nacos".to_string()
            )],
            cache_dir: "/workspaces/nacos-sdk-rust/output/failover".to_owned(),
            load_at_start: false,
            update_when_empty: false
        };
        let client = NamingClient::new(config).await;
        
        client.register_instance(Instance::new_with_defaults("test", "192.168.1.221", 8888)).await?;
        let instances = client.select_instances("test", "DEFAULT_GROUP", vec!["DEFAULT"], false).await?;
        println!("server data => \n{:?}", instances);
        tokio::time::sleep(Duration::from_secs(60 * 5)).await;

        Ok(())
    }

}