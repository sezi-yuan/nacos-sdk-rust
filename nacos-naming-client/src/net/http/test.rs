use crate::{
    net::{http::HttpNamingRemote, NamingRemote}, 
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
async fn test_http_remote_register() -> Result<()> {
    init_logger();
    let http_remote = HttpNamingRemote::new("http://192.168.1.248:8848/nacos", 8765);
    http_remote.register_instance("public", Instance::new_with_defaults("test", "192.168.1.221", 8888)).await?;
    let info = http_remote.query_instances(
        "public", "DEFAULT_GROUP@@test".to_string(), &["DEFAULT"], false
    ).await?;
    assert!(info.hosts.len() == 1, "after register; hosts len: {}", info.hosts.len());

    http_remote.deregister_instance("public", Instance::new_with_defaults("test", "192.168.1.221", 8888)).await?;
    let info = http_remote.query_instances(
        "public", "DEFAULT_GROUP@@test".to_string(), &["DEFAULT"], false
    ).await?;
    
    assert!(info.hosts.len() == 0, "after deregister; hosts len: {}", info.hosts.len());
    Ok(())
}
