use std::{io::Read, net::SocketAddr};

use serde::{Deserialize, Serialize};
use tokio::{net::UdpSocket, sync::mpsc};
use flate2::read::GzDecoder;

use crate::data::ServiceHolder;


pub const PUSH_TYPE_DOM: &'static str = "dom";
pub const PUSH_TYPE_SERVICE: &'static str = "service";
pub const PUSH_TYPE_DUMP: &'static str = "dump";

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")] 
pub struct PushPacket {
    #[serde(rename = "type")]
    push_type: String,
    last_ref_time: u64,
    data: String
}

pub struct PushReceiver {
    udp_port: u16,
    shutdown: mpsc::Sender<()>
}

impl PushReceiver {
    pub async fn new(udp_port: u16, service_holder: ServiceHolder) -> Self {
        let (tx, rx) = mpsc::channel(1);
        let receiver = Self {
            udp_port,
            shutdown: tx
        };

        let sock = UdpSocket::bind(("0.0.0.0", receiver.udp_port))
        .await
        .expect("failed to open receiver udp socket");

        tokio::spawn(Self::run(service_holder, sock, rx));
        receiver
    }

    pub async fn shutdown(&self) {
        match self.shutdown.send(()).await {
            Err(_) => log::warn!("failed to send shutdown signal to receiver"),
            _ => {}
        }
    }

    async fn run(holder: ServiceHolder, sock: UdpSocket, mut signal: mpsc::Receiver<()>) {
        let mut buf = [0; 65536];
        loop {
            let res = Self::read_from_socket(&mut buf, &sock, &mut signal).await;
            let (len, socket_addr) = match res {
                None => continue,
                Some(x) => x
            };

            let packet = match Self::parse_packet(&buf, len) {
                Some(x) => x,
                None => continue
            };

            let reply = Self::build_reply(packet, &holder).await;
            match sock.send_to(&reply[..], socket_addr).await {
                Err(error) => log::error!("push channel failed: {}", error),
                Ok(len) => log::debug!("push ack success; len: {}", len)
            }
        }
    } 

    async fn read_from_socket(
        buf: &mut [u8], 
        sock: &UdpSocket, 
        signal: &mut mpsc::Receiver<()>
    ) -> Option<(usize, SocketAddr)> {
        let res = tokio::select!{
            res = sock.recv_from(buf) => res,
            _ = signal.recv() => return None
        };

        match res {
            Err(err) => {
                log::warn!("receive illegal push message: {}", err);
                None
            },
            Ok(data) => Some(data)
        }
    }

    fn parse_packet(buf: &[u8], msg_len: usize) -> Option<PushPacket> {
        let mut gz = GzDecoder::new(&buf[..msg_len]);
        let mut buffer = String::new();
        let _ = gz.read_to_string(&mut buffer);

        log::debug!("receive push message from nacos: {}", buffer);
        match serde_json::from_str::<PushPacket>(buffer.trim()) {
            Err(err) => {
                log::warn!("receive illegal push message: {} \n{}", err, buffer);
                None
            },
            Ok(data) => Some(data)
        }
    }

    async fn build_reply(packet: PushPacket, holder: &ServiceHolder) -> Vec<u8> {
        let mut default = PushPacket {
            push_type: "unknown-ack".to_string(), 
            last_ref_time: packet.last_ref_time, 
            data: "".to_string()
        };
        let reply = match packet.push_type.as_str() {
            PUSH_TYPE_DOM | PUSH_TYPE_SERVICE => {
                let service_info = serde_json::from_str(packet.data.as_str());
                match service_info {
                    Ok(info) => holder.update_service_info(info).await,
                    Err(error) => log::error!("can not serialize push data: {}\n{}", error, packet.data)
                }
                default.push_type = "push-ack".to_string();
                default
            },
            PUSH_TYPE_DUMP => {
                let data = serde_json::to_string(&holder.get_service_info_map().await)
                .expect("failed to serialize service_holder map");
                default.data = data;
                default.push_type = "dump-ack".to_string();
                default
            },
            _ => default
        };

        serde_json::to_vec(&reply).expect("failed to serialize push ack")
    }
}
