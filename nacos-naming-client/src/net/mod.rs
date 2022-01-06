use crate::data::model::{Instance, ExpressionSelector, Service, ServiceInfo, Token, BeatAck, BeatRequest};
use crate::error::Result;
use async_trait::async_trait;

mod http;
pub use http::HttpNamingRemote;

/// 所有操作instance的service_name都是包含group信息的service_name: {group}@@{name}
#[async_trait]
pub trait NamingRemote {
    /// 登录到nacos，获取accessKey
    async fn login(&self, username: &str, password: &str) -> Result<Token>;
    /// 注册服务实例
    async fn register_instance(&self, namespace_id: &str, token: Option<String>, instance: Instance) -> Result<()>;
    /// 注销服务实例
    async fn deregister_instance(&self, namespace_id: &str, token: Option<String>, instance: Instance) -> Result<()>;
    /// 更新实例信息
    async fn update_instance(&self, namespace_id: &str, token: Option<String>, instance: Instance) -> Result<()>;
    /// 查找实例
    async fn query_instances(
        &self, namespace_id: &str, token: Option<String>, service_name: String, clusters: &[&str], healthy_only: bool
    ) -> Result<ServiceInfo>;
    /// 创建新服务
    //fn create_service(&self, );
    /// 更新服务
    //fn update_service(&self);
    /// 删除服务
    //fn delete_service(&self);
    /// 查找服务
    async fn query_service(&self, namespace_id: &str, token: Option<String>, service_name: String) -> Result<Service>;
    /// 查找所有服务
    async fn query_all_service(
        &self, 
        namespace_id: &str, token: Option<String>,
        group_name: &str, 
        selector: Option<ExpressionSelector>, 
        page_num: u32, page_size: u32
    ) -> Result<Vec<Service>>;

    async fn beat(&self, info: &BeatRequest) -> Result<BeatAck>;

    /// 订阅服务信息变化通知
    async fn subscribe(
        &self, namespace_id: &str, token: Option<String>, service_name: &str, clusters: &[&str]
    ) -> Result<()>;
    
    /// 退订服务信息变化通知
    async fn unsubscribe(
        &self, namespace_id: &str, token: Option<String>, service_name: &str, clusters: &[&str]
    ) -> Result<()>;
}

