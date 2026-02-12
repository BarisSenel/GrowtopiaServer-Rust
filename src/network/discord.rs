use axum::{
    extract::{Query, State},
    response::IntoResponse,
    http::StatusCode,
};
use serde::{Deserialize, Serialize};
use std::env;
use tracing::{info, error};
use crate::database::player::{Player, get_player_by_discord_id};
use crate::database::db_thread::DbCommand;
use crate::AppState;
use rand::{distributions::Alphanumeric, Rng};
use base64::{Engine as _, engine::general_purpose};

#[derive(Deserialize)]
pub struct CallbackQuery {
    pub code: String,
    pub state: Option<String>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct DiscordUser {
    pub id: String,
    pub username: String,
    pub email: Option<String>,
}

#[derive(Serialize, Deserialize, Debug)]
struct DiscordTokenResponse {
    access_token: String,
    token_type: String,
    expires_in: i64,
    refresh_token: Option<String>,
    scope: String,
}

pub fn get_discord_auth_url(state: Option<String>) -> String {
    let client_id = env::var("DISCORD_CLIENT_ID").unwrap_or_default();
    let redirect_uri = env::var("DISCORD_REDIRECT_URI").unwrap_or_default();

    let mut url = format!(
        "https://discord.com/api/oauth2/authorize?client_id={}&redirect_uri={}&response_type=code&scope=identify%20email",
        client_id,
        urlencoding::encode(&redirect_uri)
    );

    if let Some(s) = state {
        url.push_str(&format!("&state={}", urlencoding::encode(&s)));
    }
    url
}

pub async fn handle_discord_callback(
    State(state): State<AppState>,
    Query(query): Query<CallbackQuery>,
) -> impl IntoResponse {
    let client_id = env::var("DISCORD_CLIENT_ID").unwrap_or_default();
    let client_secret = env::var("DISCORD_CLIENT_SECRET").unwrap_or_default();
    let redirect_uri = env::var("DISCORD_REDIRECT_URI").unwrap_or_default();

    let client = reqwest::Client::new();


    let params = [
        ("client_id", client_id),
        ("client_secret", client_secret),
        ("grant_type", "authorization_code".to_string()),
        ("code", query.code),
        ("redirect_uri", redirect_uri),
    ];

    let res = match client.post("https://discord.com/api/oauth2/token")
        .form(&params)
        .send()
        .await {
            Ok(res) => res,
            Err(e) => {
                error!("Failed to exchange Discord code: {}", e);
                return (StatusCode::INTERNAL_SERVER_ERROR, "Failed to connect to Discord").into_response();
            }
        };

    if !res.status().is_success() {
        let err_body = res.text().await.unwrap_or_default();
        error!("Discord token error: {}", err_body);
        return (StatusCode::BAD_REQUEST, "Discord authorization failed").into_response();
    }

    let token_data: DiscordTokenResponse = res.json().await.unwrap();


    let user_res = match client.get("https://discord.com/api/users/@me")
        .bearer_auth(token_data.access_token)
        .send()
        .await {
            Ok(res) => res,
            Err(e) => {
                error!("Failed to fetch Discord user info: {}", e);
                return (StatusCode::INTERNAL_SERVER_ERROR, "Failed to fetch user info").into_response();
            }
        };

    let discord_user: DiscordUser = user_res.json().await.unwrap();
    info!("Discord user logged in: {} ({})", discord_user.username, discord_user.id);


    let mut player = match get_player_by_discord_id(&discord_user.id) {
        Ok(Some(p)) => p,
        Ok(None) => {

            let mut p = Player::new(&discord_user.username);
            p.discord_id = Some(discord_user.id.clone());
            p
        },
        Err(e) => {
            error!("Database error searching for player: {}", e);
            return (StatusCode::INTERNAL_SERVER_ERROR, "Database error").into_response();
        }
    };


    player.discord_username = Some(discord_user.username);
    player.email = discord_user.email;


    let ltoken: String = rand::thread_rng()
        .sample_iter(&Alphanumeric)
        .take(32)
        .map(char::from)
        .collect();
    player.ltoken = Some(ltoken.clone());


    state.db_tx.send(DbCommand::UpdatePlayer(player.clone())).ok();


    let b64_discord_id = general_purpose::STANDARD.encode(&discord_user.id);

    let html = format!(
        r#"<!DOCTYPE html>
<html>
<head>
    <title>Redirecting...</title>
</head>
<body onload="document.getElementById('redirectForm').submit();">
    <form id="redirectForm" method="POST" action="/player/growid/login/validate">
        <input type="hidden" name="growId" value="{}">
        <input type="hidden" name="password" value="{}">
        <input type="hidden" name="_token" value="{}">
    </form>
    <p>Loading... If you are not redirected, click <button onclick="document.getElementById('redirectForm').submit();">here</button>.</p>
</body>
</html>"#,
        player.name,
        ltoken,
        b64_discord_id
    );

    (StatusCode::OK, [("Content-Type", "text/html; charset=utf-8")], html).into_response()
}