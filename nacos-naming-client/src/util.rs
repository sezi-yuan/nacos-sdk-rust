use std::net::IpAddr;

pub fn grouped_service_name(service_name: &str, group_name: &str) -> String {
    format!("{}@@{}", group_name, service_name)
}

pub fn local_ip() -> IpAddr {
    let (_, addr) = local_ip_address::list_afinet_netifas().expect("can not obtain local ip")
        .into_iter()
        .filter(|(_, ipaddr)| {
            !ipaddr.is_loopback()
            && !ipaddr.is_multicast()
            && !ipaddr.is_unspecified()
        })
        .next()
        .expect("no match local ipaddr found");
    addr
}