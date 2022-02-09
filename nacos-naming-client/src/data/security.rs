use std::{sync::Arc, time::Duration};

use tokio::sync::{Mutex, broadcast};

use crate::net::NamingRemote;

use super::model::Token;

#[derive(Clone)]
pub struct AccessTokenHolder<R:NamingRemote + Sized> {
    user_name: Option<String>,
    password: Option<String>,
    remote: R,
    token: Arc<Mutex<Token>>,
    shutdown: broadcast::Sender<()>
}

impl<R: NamingRemote> AccessTokenHolder<R> {
    pub async fn get_token(&self) -> Option<String> {
        let token = self.token.lock().await;
        if token.valid() {
            Some(token.access_token.to_string())
        } else {
            None
        }
    }
    pub fn shutdown(&self) {
        let _ = self.shutdown.send(());
    }
}

impl<R: NamingRemote + Send + Clone + 'static> AccessTokenHolder<R> {
    pub async fn new(remote: R, user_name: Option<String>, password: Option<String>) -> Self {
        let (tx, _) = broadcast::channel(1);
        let token = if user_name.is_some() && password.is_some() {
            let maybe_token = remote.login(
                user_name.as_ref().expect("[security] never happen").as_str(), 
                password.as_ref().expect("[security] never happen").as_str()
            ).await;
            match maybe_token {
                Ok(token) => token,
                Err(error) => {
                    log::error!("failed to obtain token: {}", error);
                    Default::default()
                }
            }
        } else {
            Default::default()
        };

        let token_holder = Arc::new(Mutex::new(token));
        let holder = Self {
            user_name,
            password,
            token: token_holder,
            shutdown: tx,
            remote
        };
        
        holder.start();

        holder
    }

    pub fn start(&self) {
        if self.user_name.is_none() || self.password.is_none() {
            return
        }
        tokio::spawn(do_task(
            self.token.clone(),
            self.shutdown.subscribe(), 
            self.remote.clone(), 
            self.user_name.clone().expect("[token:userName]never happen"), 
            self.password.clone().expect("[token:password]never happen")
        ));
    }
}

async fn do_task(
    token_holder: Arc<Mutex<Token>>, 
    mut rx: broadcast::Receiver<()>, 
    remote: impl NamingRemote, 
    user_name: String, password: String
) {
    loop {
        let res = tokio::select!{
            res = remote.login(user_name.as_str(), password.as_str()) => res,
            _ = rx.recv() => break
        };
        
        let ttl = match res {
            Err(err) => {
                log::error!("[token] failed to obtain token, try later; cause: {}", err);
                6u64
            },
            Ok(token) => {
                let ttl = token.token_ttl;
                *token_holder.lock().await = token;
                ttl
            }
        };
        
        log::debug!("[token] obtain new token, ttl: {}", ttl);
        tokio::time::sleep(Duration::from_secs(ttl / 2)).await;
    }
}
