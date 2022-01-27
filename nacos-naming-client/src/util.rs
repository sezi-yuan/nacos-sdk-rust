pub fn grouped_service_name(service_name: &str, group_name: &str) -> String {
    format!("{}@@{}", group_name, service_name)
}
