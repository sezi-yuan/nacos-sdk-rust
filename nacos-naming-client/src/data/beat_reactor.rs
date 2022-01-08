use std::{collections::HashMap, sync::Arc, time::Duration};

use tokio::sync::{Mutex, mpsc};

use crate::{
    net::NamingRemote,
    error::Result
};

use super::{model::{Instance, BeatInfo, BeatRequest}, AccessTokenHolder};


/// 心跳检测
pub struct HeartBeatReactor<R: NamingRemote> {
    remote: R,
    task_map: Arc<Mutex<HashMap<String, mpsc::Sender<()>>>>,
    token_holder: AccessTokenHolder<R>
}

impl<R: NamingRemote + Clone + 'static> HeartBeatReactor<R> {

    pub fn new(remote: R, token_holder: AccessTokenHolder<R>) -> Self {
        HeartBeatReactor {
            remote,
            token_holder,
            task_map: Arc::new(Mutex::new(HashMap::new()))
        }
    }
    fn build_key(instance: &Instance) -> String {
        return format!(
            "{}#{}#{}", 
            instance.service_name, 
            instance.ip, 
            instance.port
        )
    }

    pub async fn add_task(&self, namespace_id: &str, instance: Instance) -> Result<()> {
        let key = Self::build_key(&instance);
        let mut signal_map = self.task_map.lock().await;
        if let Some(_) = signal_map.get(key.as_str()) {
            return Ok(());
        }
        let (tx, mut rx) = mpsc::channel(1);
        signal_map.insert(key, tx);

        let beat_info = BeatInfo {
            ip: instance.ip,
            port: instance.port,
            weight: instance.weight,
            service_name: instance.service_name.clone(),
            cluster: instance.cluster_name,
            metadata: instance.metadata
        };
        let mut request = BeatRequest {
            namespace_id: namespace_id.to_string(),
            access_token: self.token_holder.get_token().await,
            service_name: instance.service_name,
            beat: serde_json::to_string(&beat_info).expect("beat_info can not serialize"),
            beat_info,
            period: Duration::from_secs(5)
        };
        let token_holder = self.token_holder.clone();
        let remote = self.remote.clone();
        tokio::spawn(async move {
            loop {
                request.access_token = token_holder.get_token().await;
                let res = tokio::select!{
                    res = remote.beat(&request) => res,
                    _ = rx.recv() => break
                };
                
                match res {
                    Err(err) => log::error!("[beat] failed to send beat, cause: {}", err),
                    Ok(ack) => {
                        request.period = Duration::from_millis(ack.client_beat_interval - 2000);
                    }
                }
                log::debug!(
                    "[beat] service:{} millis_period: {}; sleep....", 
                    request.service_name, request.period.as_millis()
                );
                tokio::time::sleep(request.period).await;
            }
        });
        Ok(())
    }

    pub async fn remove_task(&self, _: &str, instance: Instance) {
        let key = Self::build_key(&instance);
        if let Some(tx) = self.task_map.lock().await.remove(&key) {
            let _ = tx.send(()).await;
        }
    }

    pub async fn shutdown(&self) {
        let mut map = self.task_map.lock().await;
        let txs = map.iter().collect::<Vec<_>>();
        for (_, tx) in txs {
            let _ = tx.send(()).await;
        }
        
        map.clear();
    }
}