use std::{sync::Arc, time::Duration};

use crate::{
    net::NamingRemote,
    error::Result, 
    data::{
        model::{Instance, ServiceInfo, Service, ExpressionSelector, Token, BeatAck, BeatRequest}, 
        ServiceHolder, AccessTokenHolder
    }
};
use async_trait::async_trait;
use itertools::Itertools;
use reqwest::Method;
use serde::Serialize;
use tokio::sync::Mutex;

use super::{client::HttpClient, push_receiver::PushReceiver};

const LOGIN_PATH: &str = "/v1/auth/users/login";
const INSTANCE_PATH: &str = "/v1/ns/instance";
const SERVICE_PATH: &str = "/v1/ns/service";


#[derive(Debug, Serialize)]
struct Login<'a> {
    username: &'a str,
    password: &'a str
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")] 
struct RegisterRequest {
    pub namespace_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub access_token: Option<String>,
    pub service_name: String,
    pub group_name: String,
    pub cluster_name: String,
    pub ip: String,
    pub port: u16,
    pub weight: f64,
    /// instance health status
    pub healthy: bool,
    /// If instance is enabled to accept request.
    pub enabled: bool,
    /// If instance is ephemeral.
    pub ephemeral: bool,
    /// user extended attributes.
    pub metadata: String
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")] 
struct DeregisterRequest {
    pub namespace_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub access_token: Option<String>,
    pub service_name: String,
    pub cluster_name: String,
    pub ip: String,
    pub port: u16,
    /// If instance is ephemeral.
    pub ephemeral: bool,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")] 
pub struct QueryInstanceRequest {
    pub namespace_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub access_token: Option<String>,
    pub service_name: String,
    pub group_name: String,
    /// cluster information of instance.
    pub clusters: String,
    pub udp_port: u16,
    #[serde(rename = "clientIP")]
    pub client_ip: String,
    pub healthy_only: bool
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")] 
struct QueryServiceRequest {
    pub namespace_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub access_token: Option<String>,
    pub service_name: String
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")] 
pub struct ServiceListRequest {
    pub namespace_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub access_token: Option<String>,
    pub group_name: String,
    pub page_size: u32,
    pub page_no: u32,
    pub selector: Option<String>
}   


#[derive(Clone)]
pub struct HttpNamingRemote {
    client: HttpClient,
    service_holder: ServiceHolder,
    receiver: Arc<Mutex<PushReceiver>>,
    address: Vec<String>,
    receiver_port: u16,
    client_ip: String
}

impl HttpNamingRemote {
    pub async fn new(
        addresses: Vec<String>,
        service_holder: ServiceHolder
    ) -> Self {
        let udp_port = rand::random::<u16>() % 1000 + 54951;
        let receiver = PushReceiver::new(udp_port, service_holder.clone()).await;

        let remote = Self {
            client: HttpClient::new(),
            address: addresses,
            receiver: Arc::new(Mutex::new(receiver)),
            receiver_port: udp_port,
            service_holder,
            client_ip: local_ipaddress::get().unwrap()
        };

        log::info!(
            "http naming remote, server_address: {:?}, local_ip: {}, receiver_port: {}", 
            remote.address,
            remote.client_ip,
            remote.receiver_port
        );
        remote
    }

    pub async fn shutdown(&self) {
        self.receiver.lock().await.shutdown().await
    }
}


#[async_trait]
impl NamingRemote for HttpNamingRemote {
    async fn login(&self, username: &str, password: &str) -> Result<Token> {
        self.client.request_json(
            &self.address,
            LOGIN_PATH,
            Method::POST, 
            &Login {username, password}
        ).await
    }
    /// 注册服务实例
    async fn register_instance(&self, namespace_id: &str, token: Option<String>, instance: Instance) -> Result<()> {
        self.client.request_str(
            &self.address,
            INSTANCE_PATH,
            Method::POST, 
            &RegisterRequest::from_instance(namespace_id.to_string(), token, instance),
        )
        .await
        .map(|_| ())
    }
    /// 注销服务实例
    async fn deregister_instance(&self, namespace_id: &str, token: Option<String>, instance: Instance) -> Result<()> {
        self.client.request_str(
            &self.address,
            INSTANCE_PATH,
            Method::DELETE, 
            &DeregisterRequest {
                namespace_id: namespace_id.to_string(),
                access_token: token,
                service_name: instance.service_name,
                cluster_name: instance.cluster_name,
                ip: instance.ip,
                port: instance.port,
                ephemeral: instance.ephemeral
            }
        )
        .await
        .map(|_| ())
    }
    /// 更新实例信息
    async fn update_instance(
        &self, namespace_id: &str, token: Option<String>, instance: Instance
    ) -> Result<()> {
        self.client.request_str(
            &self.address,
            INSTANCE_PATH,
            Method::PUT, 
            &RegisterRequest::from_instance(namespace_id.to_string(), token, instance),
        )
        .await
        .map(|_| ())
    }
    /// 查找实例
    async fn query_instances(
        &self, namespace_id: &str, 
        token: Option<String>, 
        service_name: String, 
        clusters: &[&str], healthy_only: bool
    ) -> Result<ServiceInfo> {
        let clusters = clusters.iter().join(",");
        self.client.request_json(
            &self.address,
            format!("{}/{}", INSTANCE_PATH, "list").as_str(),
            Method::GET, 
            &QueryInstanceRequest {
                namespace_id: namespace_id.to_string(),
                access_token: token,
                service_name,
                //TODO fixme
                group_name: "DEFAULT_GROUP".to_owned(),
                clusters,
                udp_port: self.receiver_port,
                client_ip: self.client_ip.clone(),
                healthy_only
            }
        ).await
    }
    /// 创建新服务
    //fn create_service(&self, );
    /// 更新服务
    //fn update_service(&self);
    /// 删除服务
    //fn delete_service(&self);
    /// 查找服务
    async fn query_service(
        &self, namespace_id: &str, 
        token: Option<String>, service_name: String
    ) -> Result<Service> {
        self.client.request_json(
            &self.address,
            SERVICE_PATH,
            Method::GET, 
            &QueryServiceRequest {
                namespace_id: namespace_id.to_string(),
                access_token: token,
                service_name
            }
        ).await
        
    }
    /// 查找所有服务
    async fn query_all_service(
        &self, 
        namespace_id: &str, 
        token: Option<String>,
        group_name: &str, 
        selector: Option<ExpressionSelector>, 
        page_num: u32, page_size: u32
    ) -> Result<Vec<Service>> {
        let selector = selector.map(|se| serde_json::to_string(&se)
            .expect("can not serialize selector"));
        
        self.client.request_json(
            &self.address,
            SERVICE_PATH,
            Method::GET, 
            &ServiceListRequest {
                namespace_id: namespace_id.to_string(),
                access_token: token,
                group_name: group_name.to_string(),
                page_no: page_num,
                page_size,
                selector
            }
        ).await
    }

    async fn beat(&self, info: &BeatRequest) -> Result<BeatAck> {
        self.client.request_json(
            &self.address,
            format!("{}/{}", INSTANCE_PATH, "beat").as_str(),
            Method::PUT, 
            &info
        ).await
    }

    /// 订阅服务信息变化通知
    async fn subscribe<R: NamingRemote + 'static>(
        &self, namespace_id: &str, token: AccessTokenHolder<R>,
        service_name: &str, clusters: &[&str]
    ) -> Result<()> {
        let remote = self.clone();
        let namespace_id = namespace_id.to_string();
        let service_name = service_name.to_string();
        let cluster_vec = clusters.iter().map(|cluster| cluster.to_string()).collect::<Vec<_>>();
        tokio::spawn(async move {
            let clusters = &cluster_vec.iter().map(|cluster| cluster.as_str()).collect::<Vec<_>>()[..];
            loop {
                let myabe_token = token.get_token().await;
                let service_info = remote.query_instances(
                    namespace_id.as_str(), myabe_token, service_name.clone(), clusters, false
                ).await;
                match service_info {
                    Ok(info) => remote.service_holder.update_service_info(info).await,
                    Err(error) => log::error!("failed to subscribe service: {}; cause: {}", service_name, error)
                }
                log::debug!("wait for next query: {}", service_name);
                tokio::time::sleep(Duration::from_secs(9)).await;
                log::debug!("continue to query: {}", service_name);

            }
        });
        Ok(())
    }
    
    /// 退订服务信息变化通知
    async fn unsubscribe(
        &self, _: &str, _: Option<String>, _: &str, _: &[&str]
    ) -> Result<()> {
        Ok(())
    }
}

impl RegisterRequest {
    fn from_instance(namespace_id: String, access_token: Option<String>, instance: Instance) -> RegisterRequest {
        RegisterRequest {
            namespace_id,
            access_token,
            service_name: instance.service_name,
            group_name: instance.group_name,
            cluster_name: instance.cluster_name,
            ip: instance.ip,
            port: instance.port,
            weight: instance.weight,
            healthy: instance.healthy,
            enabled: instance.enabled,
            ephemeral: instance.ephemeral,
            metadata: serde_json::to_string(&instance.metadata)
                .expect("can not serialize instance's metadata")
        }
    }
}
