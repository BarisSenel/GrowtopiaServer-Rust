use axum::{
    response::IntoResponse,
    http::{StatusCode, HeaderMap},
    extract::ConnectInfo,
};
use std::net::SocketAddr;
use tracing::info;

pub async fn server_data(
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    body: String,
) -> impl IntoResponse {

    let server_ip = std::env::var("gameserver_adress").unwrap_or_else(|_| "127.0.0.1".to_string());
    let server_port = std::env::var("gameserver_port").unwrap_or_else(|_| "17091".to_string());
    let login_url = std::env::var("host_login_url").unwrap_or_else(|_| "127.0.0.1".to_string());


    info!(
        client_ip = %addr.ip(),
        method = "POST",
        path = "/growtopia/server_data.php",
        body = body,
        "Growtopia request received"
    );

    let response_body = format!(
        "server|{}\n\
         port|{}\n\
         loginurl|{}\n\
         #maint|There is maintenance\n\
         type|1\n\
         type2|1\n\
         meta|ignoremeta\n\
         RTENDMARKERBS1001",
         server_ip,
         server_port,
         login_url
    );

    info!(
        client_ip = %addr.ip(),
        response_len = response_body.len(),
        response_body = response_body,
        "Growtopia response sent"
    );

    (
        StatusCode::OK,
        [("Content-Type", "text/plain")],
        response_body,
    )
}