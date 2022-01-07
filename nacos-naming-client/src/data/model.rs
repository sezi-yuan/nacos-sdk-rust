use std::{collections::HashMap, time::{SystemTime, Duration}, ops::Sub};

use serde::{Deserialize, Serialize};

use crate::{constants, error::RespCode, util};

fn bool_default() -> bool {
    false
}


#[derive(Debug, Serialize)]
pub struct BeatInfo {
    pub ip: String,
    pub port: u16,
    pub weight: f64,
    pub service_name: String,
    pub cluster: String,
    pub metadata: HashMap<String, String>
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")] 
pub struct BeatRequest {
    pub namespace_id: String,
    pub service_name: String,
    pub beat: String,
    #[serde(skip_serializing)]
    pub beat_info: BeatInfo,
    #[serde(skip_serializing)]
    pub period: Duration
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")] 
pub struct Token {
    pub access_token: String,
    pub token_ttl: u64
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")] 
pub struct ExpressionSelector {
    #[serde(rename = "type")]
    pub selector_type: String,
    pub expression: String
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")] 
pub struct Service {
    pub name: String,
    pub group_name: String,
    pub app_name: String,
    pub protection_threshold: f32,
    pub metadata: HashMap<String, String>
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(rename_all = "camelCase")] 
pub struct Instance {
    pub id: Option<String>,
    pub ip: String,
    pub port: u16,
    pub weight: f64,
    /// instance health status
    pub healthy: bool,
    /// If instance is enabled to accept request.
    pub enabled: bool,
    /// If instance is ephemeral.
    pub ephemeral: bool,
    /// Service information of instance.
    pub service_name: String,
    #[serde(skip)]
    pub group_name: String,
    /// cluster information of instance.
    pub cluster_name: String,
    /// user extended attributes.
    pub metadata: HashMap<String, String>
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(rename_all = "camelCase")] 
pub struct ServiceInfo {
    #[serde(rename = "name")]
    pub service_name: String,
    pub clusters: String,
    pub cache_millis: u64,
    pub hosts: Vec<Instance>,
    pub last_ref_time: u64,
    pub checksum: String,
    #[serde(rename = "allIPs", default = "bool_default")] 
    pub all_ips: bool,
    #[serde(default = "bool_default")] 
    pub reach_protection_threshold: bool
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BeatAck {
    pub client_beat_interval: u64,
    pub code: Option<RespCode>,
    pub light_beat_enabled: Option<bool>
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")] 
pub struct ServiceList {
    count: u64,
    doms: Vec<String>
}

impl Default for Token {
    fn default() -> Self {
        Token { access_token: "".to_owned(), token_ttl: 0 }
    }
}

impl Token {
    pub fn valid(&self) -> bool {
        self.token_ttl > 10000
    }
}

impl Instance {

    pub fn new_with_defaults(service_name: &str, ip: &str, port: u16) -> Instance {
        Self::new_with_required(
            service_name, constants::DEFAULT_GROUP, 
            constants::DEFAULT_CLUSTER, ip, port
        )
    }

    pub fn new_with_required(
        service_name: &str,
        group_name: &str,
        cluster_name: &str,
        ip: &str, port: u16
    ) -> Instance {
        Instance {
            id: None,
            service_name: util::grouped_service_name(service_name, group_name),
            group_name: group_name.to_string(),
            cluster_name: cluster_name.to_string(),
            ip: ip.to_string(),
            port,
            weight: 1f64,
            healthy: true,
            enabled: true,
            ephemeral: true,
            metadata: HashMap::new()
        }
    }

    pub fn get_id_uncheck(&self) -> &str {
        self.id.as_ref().expect("instance id is empty!").as_str()
    }
}

impl ServiceInfo {

    pub fn get_key(&self) -> String {
        //let service_name = self.grouped_service_name();
        Self::generate_key(self.service_name.as_str(), self.clusters.as_str())
    }

    pub fn generate_key(name: &str, clusters: &str) -> String {
        if !clusters.is_empty() {
            format!("{}{}{}", name, constants::SERVICE_INFO_SPLITER, clusters)
        } else {
            name.to_string()
        }
    }

    // fn grouped_service_name(&self) -> String {
    //     if !self.group_name.is_empty() && !self.name.contains(constants::SERVICE_INFO_SPLITER) {
    //         self.group_name.clone() + constants::SERVICE_INFO_SPLITER + self.name.as_str()
    //     } else {
    //         self.name.clone()
    //     }
    // }

    pub fn expired(&self) -> bool {
        let sub = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .expect("invalid system timestamp")
            .sub(Duration::from_millis(self.last_ref_time));

        sub > Duration::from_millis(self.cache_millis)
    }
}