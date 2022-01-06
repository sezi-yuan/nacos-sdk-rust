pub mod model;
mod security;
mod beat_reactor;
mod service_holder;

pub use beat_reactor::HeartBeatReactor;
pub use service_holder::ServiceHolder;
pub use security::AccessTokenHolder;
use self::model::ServiceInfo;

use async_trait::async_trait;

#[async_trait]
pub trait ServiceChangeListener: Send + Sync {
    async fn changed(&mut self, info: ServiceInfo);
}