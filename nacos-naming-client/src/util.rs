pub fn grouped_service_name(service_name: &str, group_name: &str) -> String {
    format!("{}@@{}", group_name, service_name)
}

pub fn local_ip() -> String {
    let ip = local_ip_address::local_ip().expect("can not obtain local ip");
    ip.to_string()
}