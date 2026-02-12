use axum::{
    response::{IntoResponse, Redirect},
    http::StatusCode,
    Json,
};
use serde::{Deserialize, Serialize};
use tracing::info;
use crate::database::player::{get_player_by_ltoken};
use base64::{Engine as _, engine::general_purpose};
use serde_json;
use serde_urlencoded;

#[derive(Deserialize)]
pub struct ValidateRequest {
    pub _token: String,
    #[serde(rename = "growId")]
    pub grow_id: String,
    pub password: String,
}

#[derive(Deserialize)]
pub struct CheckTokenRequest {
    #[serde(rename = "refreshToken")]
    pub refresh_token: String,
    #[serde(rename = "clientData")]
    pub client_data: String,
}

#[derive(Serialize)]
pub struct LoginResponse {
    pub status: String,
    pub message: String,
    pub token: String,
    pub url: String,
    #[serde(rename = "accountType")]
    pub account_type: String,
}

pub async fn validate(
    headers: axum::http::HeaderMap,
    body: axum::body::Bytes,
) -> impl IntoResponse {
    let content_type = headers.get(axum::http::header::CONTENT_TYPE).and_then(|h| h.to_str().ok()).unwrap_or_default();

    let mut params = std::collections::HashMap::new();
    if content_type.contains("application/json") {
        if let Ok(json) = serde_json::from_slice::<serde_json::Value>(&body) {
            if let Some(obj) = json.as_object() {
                for (k, v) in obj {
                    if let Some(s) = v.as_str() {
                        params.insert(k.clone(), s.to_string());
                    } else if v.is_number() {
                        params.insert(k.clone(), v.to_string());
                    }
                }
            }
        }
    } else if let Ok(form) = serde_urlencoded::from_bytes::<std::collections::HashMap<String, String>>(&body) {
        params = form;
    }

    let grow_id = params.get("growId").or_else(|| params.get("growid")).cloned().unwrap_or_default();
    let password = params.get("password").cloned().unwrap_or_default();
    let token = params.get("_token").cloned().unwrap_or_default();

    info!("Validation request for GrowID: {}", grow_id);

    let response_body = match get_player_by_ltoken(&password) {
        Ok(Some(player)) => {
            if player.name != grow_id {
                format!(
                    "{{\"status\":\"error\", \"message\":\"GrowID mismatch\", \"token\":\"\", \"url\":\"\", \"accountType\":\"growtopia\"}}"
                )
            } else {

                let effective_token = player.discord_id.as_deref().unwrap_or(&token);
                let raw_token = format!("_token={}&growId={}&password={}", effective_token, grow_id, password);
                let b64_token = general_purpose::STANDARD.encode(raw_token);

                format!(
                    "{{\"status\":\"success\", \"message\":\"Account Validated.\", \"token\":\"{}\", \"url\":\"\", \"accountType\":\"growtopia\"}}",
                    b64_token
                )
            }
        },
        _ => "{\"status\":\"error\", \"message\":\"Account not valid.\", \"token\":\"\", \"accountType\":\"growtopia\"}".to_string()
    };

    (
        StatusCode::OK,
        [
            ("Content-Type", "text/html; charset=utf-8"),
            ("Access-Control-Allow-Origin", "*"),
            ("Access-Control-Expose-Headers", "*"),
            ("Vary", "Accept-Encoding"),
            ("Vary", "Origin, Access-Control-Request-Method, Access-Control-Request-Headers"),
        ],
        response_body
    ).into_response()
}

pub async fn check_token(
    headers: axum::http::HeaderMap,
    body: axum::body::Bytes,
) -> impl IntoResponse {
    let content_type = headers.get(axum::http::header::CONTENT_TYPE).and_then(|h| h.to_str().ok()).unwrap_or_default();

    let (refresh_token, client_data) = if content_type.contains("application/json") {
        match serde_json::from_slice::<CheckTokenRequest>(&body) {
            Ok(req) => (req.refresh_token, req.client_data),
            Err(_) => return (StatusCode::BAD_REQUEST, "Invalid JSON").into_response(),
        }
    } else {
        match serde_urlencoded::from_bytes::<CheckTokenRequest>(&body) {
            Ok(req) => (req.refresh_token, req.client_data),
            Err(_) => return (StatusCode::BAD_REQUEST, "Invalid Form Data").into_response(),
        }
    };

    let decoded = match general_purpose::STANDARD.decode(&refresh_token) {
        Ok(d) => String::from_utf8_lossy(&d).to_string(),
        Err(_) => return StatusCode::BAD_REQUEST.into_response(),
    };

    let b64_client_data = general_purpose::STANDARD.encode(&client_data);

    let new_token_content = if let Some(pos) = decoded.find("_token=") {
        if let Some(end_pos) = decoded[pos..].find('&') {
            format!("{}_token={}{}", &decoded[..pos], b64_client_data, &decoded[pos + end_pos..])
        } else {
            format!("{}_token={}", &decoded[..pos], b64_client_data)
        }
    } else {
        decoded
    };

    let new_token = general_purpose::STANDARD.encode(new_token_content);

    let success_json = format!(
        "{{\"status\":\"success\", \"message\":\"Token is valid.\", \"token\":\"{}\", \"url\":\"\", \"accountType\":\"growtopia\"}}",
        new_token
    );

    (
        StatusCode::OK,
        [
            ("Content-Type", "text/html; charset=utf-8"),
            ("Access-Control-Allow-Origin", "*"),
            ("Access-Control-Expose-Headers", "*"),
            ("Vary", "Accept-Encoding"),
            ("Vary", "Origin, Access-Control-Request-Method, Access-Control-Request-Headers"),
        ],
        success_json
    ).into_response()
}

pub async fn dashboard() -> impl IntoResponse {
    let discord_url = crate::network::discord::get_discord_auth_url(None);

    let html = format!(
        r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>Growtopia - Login</title>
    <script src="https:
    <link rel="stylesheet" href="https://cdnjs.cloudflare.com/ajax/libs/font-awesome/6.0.0-beta3/css/all.min.css">
</head>
<body class="flex flex-col items-center justify-center h-screen bg-transparent">
    <div class="fixed inset-0 flex justify-center items-start pt-8">
        <div id="loginModal" class="relative bg-[#2b4d6d] border-4 border-[#87b8cc] shadow-lg p-8 w-3/5 max-w-lg sm:w-4/5 xs:w-11/12 rounded-lg flex flex-col justify-center transition-all duration-700 ease-out">
            <p class="text-white text-center font-bold text-2xl mb-6">Growtopia Login</p>

            <div class="flex flex-col items-center gap-4">
                <p class="text-white text-sm text-center opacity-80 mb-2">Login with your Discord account to continue.</p>

                <a href="{}" class="flex items-center justify-center w-full px-6 py-4 bg-[#5865F2] hover:bg-[#4752C4] text-white font-bold rounded-lg shadow-xl transition duration-300 transform hover:scale-105">
                    <i class="fab fa-discord mr-3 text-2xl"></i> Login with Discord
                </a>

                <p class="text-[#87b8cc] text-[10px] mt-4 text-center">By logging in, you agree to our terms of service.</p>
            </div>
        </div>
    </div>
</body>
</html>"#,
        discord_url
    );

    (StatusCode::OK, [("Content-Type", "text/html")], html).into_response()
}

pub async fn login_discord() -> impl IntoResponse {
    Redirect::to(&crate::network::discord::get_discord_auth_url(None))
}