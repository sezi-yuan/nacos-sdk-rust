use itertools::Itertools;

use crate::{
    config::NamingConfig, 
    net::NamingRemote, 
    error::Result, 
    data::{
        ServiceHolder, HeartBeatReactor, 
        model::*, ServiceChangeListener, 
    }, util, HttpNamingRemote
};

pub struct NamingClient<R: NamingRemote> {
    config: NamingConfig,
    remote: R,
    service_holder: ServiceHolder,
    beat_reactor: HeartBeatReactor<R>
}

impl NamingClient<HttpNamingRemote> {
    pub async fn new(config: NamingConfig) -> Self {
        let service_holder = match ServiceHolder::new(
            config.cache_dir.as_str(), config.update_when_empty, config.load_at_start
        ).await {
            Ok(s) => s,
            Err(error) => panic!("{}", error)
        };
        let mut server_list = vec![];
        for server in config.server_list.iter() {
            server_list.push(server.to_string());
        }
        let remote = HttpNamingRemote::new(server_list, service_holder.clone()).await;
        let beat_reactor = HeartBeatReactor::new(remote.clone());
        Self {
            config, remote, service_holder, beat_reactor
        }
    }

    pub async fn shutdown(&self) {
        self.remote.shutdown().await;
        self.beat_reactor.shutdown().await
    }
}

impl<R: NamingRemote+Clone+Send+'static> NamingClient<R> {
    /// register a instance
    pub async fn register_instance(&self, ins: Instance) -> Result<()> {
        let namespace_id = self.config.namespace_id.as_str();
        self.remote.register_instance(namespace_id, ins.clone()).await?;
        self.beat_reactor.add_task(namespace_id, ins).await
    }

    /// deregister a instance
    pub async fn deregister_instance(&self, instance: Instance) -> Result<()> {
        let namespace_id = self.config.namespace_id.as_str();
        self.beat_reactor.remove_task(namespace_id, instance.clone()).await;
        self.remote.deregister_instance(namespace_id, instance).await
    }

    /// Get all instances within specified clusters of a service.
    /// auto subuscribe
    pub async fn select_instances<'a, C: AsRef<[&'a str]>>(
        &self,
        service_name: &str,
        group_name: &str,
        clusters: C,
        healthy: bool
    ) -> Result<Vec<Instance>> {
        let namespace_id = self.config.namespace_id.as_str();
        let service_name = util::grouped_service_name(
            service_name, group_name
        );
        let cluster_vec = clusters.as_ref();
        let service_info = self.service_holder.get_service_info(
            service_name.as_str(), cluster_vec
        ).await;
        
        let service_info = match service_info {
            Some(info) => info,
            None => {
                let info = self.remote.query_instances(
                    namespace_id, service_name.to_string(), cluster_vec, false
                ).await?;
                self.service_holder.update_service_info(info).await;
                self.service_holder.get_service_info(
                    service_name.as_str(), cluster_vec
                ).await.expect("[service_holder]never happen")
            }
        };

        let ret = service_info.hosts.into_iter()
            .filter(|host| host.healthy == healthy)
            .filter(|host| host.enabled)
            .filter(|host| host.weight > 0f64)
            .collect::<Vec<_>>();

        Ok(ret)
    }

    /// Subscribe service to receive events of instances alteration.
    pub async fn subscribe<'a, C: AsRef<[&'a str]>, L: ServiceChangeListener + 'static>(
        &self,
        service_name: &str,
        group_name: &str,
        clusters: C,
        listener: L
    ) -> Result<()> {
        let namespace_id = self.config.namespace_id.as_str();
        let service_name = util::grouped_service_name(
            service_name, group_name
        );
        let cluster_vec = clusters.as_ref();
        self.remote.subscribe(namespace_id, service_name.as_str(), cluster_vec).await?;

        self.service_holder.register_subscribe(
            service_name,
            cluster_vec.into_iter().join(","), 
            Box::new(listener)
        ).await;
        Ok(())
    }

    /// Unsubscribe event listener of service.
    pub async fn unsubscribe<'a, C: AsRef<[&'a str]>>(
        &self,
        service_name: &str,
        group_name: &str,
        clusters: C
    ) -> Result<()> {
        let namespace_id = self.config.namespace_id.as_str();
        let service_name = util::grouped_service_name(
            service_name, group_name
        );
        let cluster_vec = clusters.as_ref();
        self.remote.unsubscribe(namespace_id, service_name.as_str(), cluster_vec).await
    }
}