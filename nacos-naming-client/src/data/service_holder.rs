use std::{collections::HashMap, sync::Arc, path::{Path, PathBuf}};

use itertools::Itertools;
use tokio::sync::Mutex;

use crate::{error::{Error, Result}, model::Instance};

use super::{model::ServiceInfo, ServiceChangeListener};


/// 服务缓存
#[derive(Clone)]
pub struct ServiceHolder {
    service_map: Arc<Mutex<HashMap<String, ServiceInfo>>>,
    callbacks: Arc<Mutex<HashMap<String, Vec<Box<dyn ServiceChangeListener>>>>>,
    cache_dir: PathBuf,
    update_when_empty: bool
}

impl ServiceHolder {

    pub async fn new(cache_dir: impl AsRef<Path>, update_when_empty: bool, load_at_start: bool) -> Result<Self> {
        let path = cache_dir.as_ref();
        if !path.exists() {
            tokio::fs::create_dir_all(path).await
                .map_err(|err| Error::Fs("can not create cache dir".to_owned(), err))?;
        }

        let holder = ServiceHolder {
            service_map: Arc::new(Mutex::new(HashMap::new())),
            callbacks: Arc::new(Mutex::new(HashMap::new())),
            cache_dir: cache_dir.as_ref().to_path_buf(),
            update_when_empty
        };

        if load_at_start {
            holder.load_from_disk().await?;
        }

        Ok(holder)
    }

    async fn load_from_disk(&self) -> Result<()> {
        let dir = self.cache_dir.as_path();
        let map: HashMap<String, ServiceInfo> = nacos_sdk_core::cache::read_dir(dir).await?;

        let mut info_map = self.service_map.lock().await;
        info_map.extend(map.into_iter());
        Ok(())
    }

    pub async fn get_service_info(
        &self,
        service_name: &str,
        clusters: &[&str]
    ) -> Option<ServiceInfo> {
        let clusters = clusters.into_iter().join(",");
        let key = ServiceInfo::generate_key(service_name, clusters.as_str());
        self.service_map.lock().await.get(key.as_str()).map(|data| data.clone())
    }

    pub async fn update_service_info(
        &self,
        service_info: ServiceInfo
    ) {
        let key = service_info.get_key();
        let old = self.service_map.lock().await.insert(key.clone(), service_info.clone());
        let push_instance = Self::diff_instance(old.map(|x|x.hosts), service_info.hosts.clone());
        let mut callbacks = self.callbacks.lock().await;
        let maybe_callbacks = callbacks.get_mut(key.as_str());
        let mut default = vec![];
        let vec = maybe_callbacks.unwrap_or(&mut default);
        for listener in vec {
            listener.changed(key.as_str(), push_instance.clone()).await;
            log::debug!("service change has been notified: {}", key.as_str());
        }
        let result = nacos_sdk_core::cache::write_file(
            &service_info, self.cache_dir.clone(), key.as_str()
        ).await;

        if let Err(error) = result {
            log::warn!("can not write service_info cache to disk: {}", error);
        }
        log::debug!("service change has write to disk successed: {}", service_info.service_name);
    }

    fn diff_instance(old: Option<Vec<Instance>>, mut new: Vec<Instance>) -> Vec<Instance> {
        let old = match old {
            None => return new,
            Some(list) => list
        };
        if old.is_empty() {
            return new;
        }

        for mut old_instance in old {
            match Self::get_same_instance(&new, &old_instance) {
                // 这里没有剔除已经在本地生效的instance
                // 调用者应该做去重处理来避免把该instance的权重错误加重
                None => {
                    // 如果new中没有该instance但是old中有，那么应该设置为剔除
                    old_instance.enabled = false;
                    new.push(old_instance);
                },
                // 其他情况不处理
                _ => {}
            };
        }

        new
    }

    fn get_same_instance<'a>(new: &'a Vec<Instance>, old: &'a Instance) -> Option<&'a Instance> {
        for item in new.iter() {
            if item.key() == old.key() {
                return Some(item)
            }
        }
        None
    }

    pub async fn register_subscribe(
        &self, 
        service_name: String, clusters: String, 
        listener: Box<dyn ServiceChangeListener>
    ) {
        let key = ServiceInfo::generate_key(service_name.as_str(), clusters.as_str());
        let mut callback_map = self.callbacks.lock().await;
        if !callback_map.contains_key(key.as_str()) {
            callback_map.insert(key.clone(), vec![]);
        }

        let _ = callback_map.get_mut(key.as_str()).map(|v|v.push(listener));
    }

    pub async fn get_service_info_map(&self) -> HashMap<String, ServiceInfo> {
        self.service_map.lock().await.clone()
    }
}

