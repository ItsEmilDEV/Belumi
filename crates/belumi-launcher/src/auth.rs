use serde::Deserialize;
use serde_json::{Value, json};
use std::time::{Duration, Instant};

const DEVICE_CODE_URL: &str = "https://login.microsoftonline.com/consumers/oauth2/v2.0/devicecode";
const TOKEN_URL: &str = "https://login.microsoftonline.com/consumers/oauth2/v2.0/token";
const XBL_URL: &str = "https://user.auth.xboxlive.com/user/authenticate";
const XSTS_URL: &str = "https://xsts.auth.xboxlive.com/xsts/authorize";
const MC_LOGIN_URL: &str = "https://api.minecraftservices.com/authentication/login_with_xbox";
const MC_ENTITLEMENTS_URL: &str = "https://api.minecraftservices.com/entitlements/mcstore";
const MC_PROFILE_URL: &str = "https://api.minecraftservices.com/minecraft/profile";
const SCOPE: &str = "XboxLive.signin offline_access";

#[derive(Debug, Clone, Deserialize)]
pub struct DeviceCode {
    pub device_code: String,
    pub user_code: String,
    pub verification_uri: String,
    pub expires_in: u64,
    #[serde(default = "default_interval")]
    pub interval: u64,
}
fn default_interval() -> u64 {
    5
}

#[derive(Debug, Clone)]
pub struct MsTokens {
    pub access_token: String,
    pub refresh_token: String,
}

#[derive(Debug, Clone)]
pub struct Account {
    pub name: String,
    pub uuid: String,
    pub access_token: String,
    pub refresh_token: String,
    pub expires_at: u64,
}

fn agent() -> ureq::Agent {
    ureq::AgentBuilder::new()
        .timeout(Duration::from_secs(30))
        .build()
}

fn describe(error: ureq::Error) -> (u16, Value) {
    match error {
        ureq::Error::Status(code, response) => {
            (code, response.into_json::<Value>().unwrap_or(Value::Null))
        }
        ureq::Error::Transport(t) => (0, json!({ "transport": t.to_string() })),
    }
}

fn network_error(body: &Value) -> String {
    format!(
        "Network error: {}",
        body["transport"].as_str().unwrap_or("unreachable")
    )
}

pub fn start_device_code(client_id: &str) -> Result<DeviceCode, String> {
    agent()
        .post(DEVICE_CODE_URL)
        .send_form(&[("client_id", client_id), ("scope", SCOPE)])
        .map_err(|e| {
            let (code, body) = describe(e);
            if code == 0 {
                network_error(&body)
            } else {
                format!(
                    "Microsoft refused the client ID ({code}): {}. Check the client ID and that \
                     'Allow public client flows' is enabled for the app.",
                    body["error_description"].as_str().unwrap_or("no details")
                )
            }
        })?
        .into_json()
        .map_err(|e| format!("Unreadable device-code reply: {e}"))
}

pub fn wait_for_sign_in(
    client_id: &str,
    code: &DeviceCode,
    cancelled: &dyn Fn() -> bool,
) -> Result<MsTokens, String> {
    let deadline = Instant::now() + Duration::from_secs(code.expires_in);
    let mut interval = code.interval.max(1);
    loop {
        if cancelled() {
            return Err("Cancelled".into());
        }
        if Instant::now() > deadline {
            return Err("The sign-in code expired. Start again.".into());
        }
        std::thread::sleep(Duration::from_secs(interval));
        let reply = agent().post(TOKEN_URL).send_form(&[
            ("grant_type", "urn:ietf:params:oauth:grant-type:device_code"),
            ("client_id", client_id),
            ("device_code", &code.device_code),
        ]);
        match reply {
            Ok(response) => return tokens_from(response.into_json().map_err(|e| e.to_string())?),
            Err(e) => {
                let (status, body) = describe(e);
                match body["error"].as_str() {
                    Some("authorization_pending") => {}
                    Some("slow_down") => interval += 5,
                    Some("authorization_declined") => return Err("Sign-in was declined.".into()),
                    Some("expired_token") | Some("bad_verification_code") => {
                        return Err("The sign-in code expired. Start again.".into());
                    }
                    _ if status == 0 => return Err(network_error(&body)),
                    other => {
                        return Err(format!(
                            "Microsoft sign-in failed ({status}, {})",
                            other.unwrap_or("unknown error")
                        ));
                    }
                }
            }
        }
    }
}

fn tokens_from(body: Value) -> Result<MsTokens, String> {
    let get = |key: &str| {
        body[key]
            .as_str()
            .map(str::to_owned)
            .ok_or_else(|| format!("Microsoft's reply has no {key}"))
    };
    Ok(MsTokens {
        access_token: get("access_token")?,
        refresh_token: get("refresh_token")?,
    })
}

pub fn refresh(client_id: &str, refresh_token: &str) -> Result<MsTokens, String> {
    let reply = agent().post(TOKEN_URL).send_form(&[
        ("grant_type", "refresh_token"),
        ("client_id", client_id),
        ("refresh_token", refresh_token),
        ("scope", SCOPE),
    ]);
    match reply {
        Ok(response) => tokens_from(response.into_json().map_err(|e| e.to_string())?),
        Err(e) => {
            let (status, body) = describe(e);
            if status == 0 {
                Err(network_error(&body))
            } else if body["error"] == "invalid_grant" {
                Err("The saved sign-in is no longer valid. Sign in again.".into())
            } else {
                Err(format!("Could not renew the sign-in ({status})"))
            }
        }
    }
}

pub fn minecraft_account(ms: &MsTokens) -> Result<Account, String> {
    let (xbl_token, uhs) = xbox_token(
        XBL_URL,
        json!({
            "Properties": {
                "AuthMethod": "RPS",
                "SiteName": "user.auth.xboxlive.com",
                "RpsTicket": format!("d={}", ms.access_token),
            },
            "RelyingParty": "http://auth.xboxlive.com",
            "TokenType": "JWT",
        }),
    )?;
    let (xsts, _) = xbox_token(
        XSTS_URL,
        json!({
            "Properties": { "SandboxId": "RETAIL", "UserTokens": [xbl_token] },
            "RelyingParty": "rp://api.minecraftservices.com/",
            "TokenType": "JWT",
        }),
    )?;
    let login = agent()
        .post(MC_LOGIN_URL)
        .send_json(json!({ "identityToken": format!("XBL3.0 x={uhs};{xsts}") }))
        .map_err(|e| {
            let (status, body) = describe(e);
            match status {
                0 => network_error(&body),
                403 => APP_NOT_APPROVED.into(),
                _ => format!("Minecraft login failed ({status})"),
            }
        })?
        .into_json::<Value>()
        .map_err(|e| e.to_string())?;
    let access_token = login["access_token"]
        .as_str()
        .ok_or("Minecraft's reply has no access token")?
        .to_owned();

    let bearer = format!("Bearer {access_token}");
    let entitlements = agent()
        .get(MC_ENTITLEMENTS_URL)
        .set("Authorization", &bearer)
        .call()
        .map_err(|e| format!("Could not check ownership ({})", describe(e).0))?
        .into_json::<Value>()
        .map_err(|e| e.to_string())?;
    if !owns_minecraft(&entitlements) {
        return Err("This account doesn't own Minecraft: Java Edition.".into());
    }
    let profile = match agent()
        .get(MC_PROFILE_URL)
        .set("Authorization", &bearer)
        .call()
    {
        Ok(r) => r.into_json::<Value>().map_err(|e| e.to_string())?,
        Err(ureq::Error::Status(404, _)) => {
            return Err("Create your Minecraft profile on minecraft.net first.".into());
        }
        Err(e) => return Err(format!("Could not read the profile ({})", describe(e).0)),
    };
    Ok(Account {
        name: profile["name"]
            .as_str()
            .ok_or("No profile name")?
            .to_owned(),
        uuid: profile["id"].as_str().ok_or("No profile id")?.to_owned(),
        access_token,
        refresh_token: ms.refresh_token.clone(),
        expires_at: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |d| d.as_secs())
            + login["expires_in"].as_u64().unwrap_or(3600),
    })
}

pub const APP_NOT_APPROVED: &str = "Mojang has not approved this app's client ID yet \
(Minecraft services answered 403 'Invalid app registration'). Request the review at \
https://aka.ms/mce-reviewappid and sign in again once it is approved.";

fn xbox_token(url: &str, body: Value) -> Result<(String, String), String> {
    let reply = agent().post(url).send_json(body).map_err(|e| {
        let (status, body) = describe(e);
        if status == 0 {
            network_error(&body)
        } else {
            xerr_message(body["XErr"].as_u64())
                .unwrap_or_else(|| format!("Xbox Live refused ({status})"))
        }
    })?;
    let reply: Value = reply.into_json().map_err(|e| e.to_string())?;
    let token = reply["Token"].as_str().ok_or("Xbox reply has no token")?;
    let uhs = reply["DisplayClaims"]["xui"][0]["uhs"]
        .as_str()
        .ok_or("Xbox reply has no user hash")?;
    Ok((token.to_owned(), uhs.to_owned()))
}

pub fn xerr_message(code: Option<u64>) -> Option<String> {
    Some(
        match code? {
            2148916233 => "This Microsoft account has no Xbox profile yet. Sign in once at xbox.com to create it.",
            2148916235 => "Xbox Live is not available in this account's country.",
            2148916236 | 2148916237 => "This account needs adult verification (Xbox Live).",
            2148916238 => "This is a child account: an adult must add it to a Microsoft family first.",
            other => return Some(format!("Xbox Live refused the sign-in (XErr {other})")),
        }
        .to_owned(),
    )
}

pub fn owns_minecraft(entitlements: &Value) -> bool {
    entitlements["items"].as_array().is_some_and(|items| {
        items.iter().any(|i| {
            matches!(
                i["name"].as_str(),
                Some("game_minecraft") | Some("product_minecraft")
            )
        })
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ownership_needs_a_java_entitlement() {
        assert!(owns_minecraft(
            &json!({"items": [{"name": "product_minecraft"}]})
        ));
        assert!(owns_minecraft(
            &json!({"items": [{"name": "x"}, {"name": "game_minecraft"}]})
        ));
        assert!(!owns_minecraft(
            &json!({"items": [{"name": "game_minecraft_bedrock"}]})
        ));
        assert!(!owns_minecraft(&json!({"items": []})));
        assert!(!owns_minecraft(&json!({})));
    }

    #[test]
    fn xbox_errors_have_messages() {
        assert!(
            xerr_message(Some(2148916233))
                .unwrap()
                .contains("no Xbox profile")
        );
        assert!(xerr_message(Some(2148916238)).unwrap().contains("child"));
        assert!(xerr_message(Some(1)).unwrap().contains("XErr 1"));
        assert_eq!(xerr_message(None), None);
    }

    #[test]
    fn token_replies_need_both_tokens() {
        let ok = tokens_from(json!({"access_token": "a", "refresh_token": "r"})).unwrap();
        assert_eq!(
            (ok.access_token.as_str(), ok.refresh_token.as_str()),
            ("a", "r")
        );
        assert!(
            tokens_from(json!({"access_token": "a"}))
                .unwrap_err()
                .contains("refresh_token")
        );
    }

    #[test]
    fn device_codes_default_their_poll_interval() {
        let code: DeviceCode = serde_json::from_value(json!({
            "device_code": "d", "user_code": "U", "verification_uri": "https://x", "expires_in": 900
        }))
        .unwrap();
        assert_eq!(code.interval, 5);
    }
}
