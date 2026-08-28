//! 短信推送服务模块
//!
//! 为 PushPlus、Server酱 Turbo、PushDeer、Bark、ntfy、钉钉群机器人等
//! 轻量推送服务提供统一的短信转发入口。

use std::sync::Arc;

use base64::engine::general_purpose::STANDARD as BASE64_STANDARD;
use base64::Engine;
use chrono::Utc;
use hmac::{Hmac, Mac};
use reqwest::{Client, RequestBuilder, StatusCode};
use serde_json::{json, Value};
use sha2::Sha256;

use crate::config::{ConfigManager, SmsPushConfig, SmsPushProvider};
use crate::db::SmsMessage;

const DINGTALK_DEFAULT_ENDPOINT: &str = "https://oapi.dingtalk.com/robot/send";

pub struct SmsPushSender {
    client: Client,
    config_manager: Arc<ConfigManager>,
}

impl SmsPushSender {
    pub fn new(config_manager: Arc<ConfigManager>) -> Self {
        Self {
            client: Client::builder()
                .timeout(std::time::Duration::from_secs(10))
                .build()
                .expect("Failed to create HTTP client"),
            config_manager,
        }
    }

    fn get_config(&self) -> SmsPushConfig {
        self.config_manager.get_sms_push()
    }

    pub async fn forward_sms(&self, message: &SmsMessage, own_number: &str) -> Result<(), String> {
        let config = self.get_config();

        if !config.enabled {
            return Ok(());
        }

        let title = render_sms_push_template(&config.title_template, message, own_number);
        let body = render_sms_push_template(&config.body_template, message, own_number);

        self.send_with_config(&config, &title, &body).await.map(|_| ())
    }

    pub async fn test_sms_push(&self) -> Result<String, String> {
        let config = self.get_config();

        if !config.enabled {
            return Err("短信推送服务未启用".to_string());
        }

        let test_message = SmsMessage {
            id: 0,
            direction: "incoming".to_string(),
            phone_number: "+8613800138000".to_string(),
            content: "这是一条测试短信 (SMS Push Test)".to_string(),
            timestamp: Utc::now().format("%Y-%m-%d %H:%M:%S").to_string(),
            status: "received".to_string(),
            pdu: None,
        };

        let title = render_sms_push_template(&config.title_template, &test_message, "+8613912345678");
        let body = render_sms_push_template(&config.body_template, &test_message, "+8613912345678");

        self.send_with_config(&config, &title, &body).await
    }

    async fn send_with_config(
        &self,
        config: &SmsPushConfig,
        title: &str,
        body: &str,
    ) -> Result<String, String> {
        validate_config(config)?;

        let request = build_request(&self.client, config, title, body)?;
        let response = request
            .send()
            .await
            .map_err(|e| format!("Failed to send SMS push: {}", e))?;

        let status = response.status();
        let response_body = response.text().await.unwrap_or_default();
        validate_provider_response(config.provider, status, &response_body)?;

        let response_preview = preview_response(&response_body);
        if response_preview.is_empty() {
            Ok(format!("短信推送测试成功 (status: {})", status))
        } else {
            Ok(format!("短信推送测试成功 (status: {}) {}", status, response_preview))
        }
    }
}

fn validate_config(config: &SmsPushConfig) -> Result<(), String> {
    let credential = config.credential.trim();
    let topic = config.topic.trim();

    match config.provider {
        SmsPushProvider::Pushplus
        | SmsPushProvider::Serverchan
        | SmsPushProvider::Pushdeer
        | SmsPushProvider::Bark => {
            if credential.is_empty() {
                return Err("当前推送服务缺少凭证".to_string());
            }
        }
        SmsPushProvider::Ntfy => {
            if topic.is_empty() {
                return Err("ntfy 主题不能为空".to_string());
            }
        }
        SmsPushProvider::Dingtalk => {
            if credential.is_empty() {
                return Err("钉钉机器人 access_token 不能为空".to_string());
            }
            if config.sign_enabled && config.secret.trim().is_empty() {
                return Err("安全设置为加签时，加签密钥不能为空".to_string());
            }
        }
    }

    Ok(())
}

fn build_request(
    client: &Client,
    config: &SmsPushConfig,
    title: &str,
    body: &str,
) -> Result<RequestBuilder, String> {
    let credential = config.credential.trim();
    let topic = config.topic.trim();

    match config.provider {
        SmsPushProvider::Pushplus => {
            let endpoint = resolve_endpoint(&config.server_url, "https://www.pushplus.plus/send");
            let mut payload = json!({
                "token": credential,
                "title": title,
                "content": body,
                "template": "markdown",
            });

            if !topic.is_empty() {
                payload["topic"] = json!(topic);
            }

            Ok(client.post(endpoint).json(&payload))
        }
        SmsPushProvider::Serverchan => {
            let base = resolve_base_url(&config.server_url, "https://sctapi.ftqq.com");
            let endpoint = format!("{}/{}.send", base, credential);

            Ok(client.post(endpoint).form(&[
                ("title", title),
                ("text", title),
                ("desp", body),
            ]))
        }
        SmsPushProvider::Pushdeer => {
            let endpoint = resolve_endpoint(&config.server_url, "https://api2.pushdeer.com/message/push");
            Ok(client.post(endpoint).form(&[
                ("pushkey", credential),
                ("text", title),
                ("desp", body),
                ("type", "markdown"),
            ]))
        }
        SmsPushProvider::Bark => {
            let endpoint = resolve_endpoint(&config.server_url, "https://api.day.app/push");
            let mut form_fields = vec![
                ("device_key", credential),
                ("title", title),
                ("body", body),
            ];

            if !topic.is_empty() {
                form_fields.push(("group", topic));
            }

            Ok(client.post(endpoint).form(&form_fields))
        }
        SmsPushProvider::Ntfy => {
            let endpoint = format!("{}/", resolve_base_url(&config.server_url, "https://ntfy.sh"));
            let mut request = client.post(endpoint).json(&json!({
                "topic": topic,
                "title": title,
                "message": body,
                "markdown": true,
            }));

            if !credential.is_empty() {
                request = request.bearer_auth(credential);
            }

            Ok(request)
        }
        SmsPushProvider::Dingtalk => {
            let endpoint = resolve_endpoint(&config.server_url, DINGTALK_DEFAULT_ENDPOINT);
            let content = if title.is_empty() {
                body.to_string()
            } else if body.is_empty() {
                title.to_string()
            } else {
                format!("{}\n{}", title, body)
            };

            let mut query: Vec<(&str, String)> = vec![("access_token", credential.to_string())];
            if config.sign_enabled {
                let timestamp = Utc::now().timestamp_millis().to_string();
                let sign = dingtalk_sign(config.secret.trim(), &timestamp)?;
                query.push(("timestamp", timestamp));
                query.push(("sign", sign));
            }

            Ok(client
                .post(endpoint)
                .query(&query)
                .json(&json!({
                    "msgtype": "text",
                    "text": { "content": content },
                })))
        }
    }
}

/// 钉钉群机器人加签: base64(HmacSHA256(timestamp + "\n" + secret, secret))
fn dingtalk_sign(secret: &str, timestamp: &str) -> Result<String, String> {
    let mut mac = Hmac::<Sha256>::new_from_slice(secret.as_bytes())
        .map_err(|_| "钉钉加签密钥无效".to_string())?;
    mac.update(format!("{}\n{}", timestamp, secret).as_bytes());
    Ok(BASE64_STANDARD.encode(mac.finalize().into_bytes()))
}

fn validate_provider_response(
    provider: SmsPushProvider,
    status: StatusCode,
    body: &str,
) -> Result<(), String> {
    if !status.is_success() {
        return Err(format!(
            "推送服务返回错误状态 {}{}",
            status,
            format_body_suffix(body),
        ));
    }

    let trimmed = body.trim();
    if trimmed.is_empty() {
        return Ok(());
    }

    let value = match serde_json::from_str::<Value>(trimmed) {
        Ok(value) => value,
        Err(_) => return Ok(()),
    };

    match provider {
        SmsPushProvider::Pushplus | SmsPushProvider::Bark => {
            if let Some(code) = value.get("code").and_then(Value::as_i64) {
                if code != 200 {
                    return Err(extract_provider_error("推送服务返回失败", &value));
                }
            }
        }
        SmsPushProvider::Serverchan | SmsPushProvider::Pushdeer => {
            if let Some(code) = value.get("code").and_then(Value::as_i64) {
                if code != 0 && code != 200 {
                    return Err(extract_provider_error("推送服务返回失败", &value));
                }
            }
        }
        SmsPushProvider::Dingtalk => {
            if let Some(errcode) = value.get("errcode").and_then(Value::as_i64) {
                if errcode != 0 {
                    let reason = value
                        .get("errmsg")
                        .and_then(Value::as_str)
                        .unwrap_or("未知错误");
                    return Err(format!(
                        "钉钉机器人返回失败 (errcode: {}): {}",
                        errcode, reason
                    ));
                }
            }
        }
        SmsPushProvider::Ntfy => {}
    }

    Ok(())
}

fn extract_provider_error(prefix: &str, value: &Value) -> String {
    let message = value
        .get("msg")
        .or_else(|| value.get("message"))
        .or_else(|| value.get("error"))
        .and_then(Value::as_str)
        .unwrap_or("未知错误");

    format!("{}: {}", prefix, message)
}

fn render_sms_push_template(template: &str, message: &SmsMessage, own_number: &str) -> String {
    template
        .replace("{{own_number}}", own_number)
        .replace("{{id}}", &message.id.to_string())
        .replace("{{phone_number}}", &message.phone_number)
        .replace("{{content}}", &message.content)
        .replace("{{direction}}", &message.direction)
        .replace("{{timestamp}}", &message.timestamp)
        .replace("{{status}}", &message.status)
        .replace("{{sender}}", &message.phone_number)
        .replace("{{message}}", &message.content)
        .replace("{{time}}", &message.timestamp)
}

fn resolve_endpoint(input: &str, default: &str) -> String {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        default.to_string()
    } else {
        trimmed.trim_end_matches('/').to_string()
    }
}

fn resolve_base_url(input: &str, default: &str) -> String {
    resolve_endpoint(input, default)
}

fn preview_response(body: &str) -> String {
    let trimmed = body.trim();
    if trimmed.is_empty() {
        return String::new();
    }

    let mut preview = trimmed.chars().take(120).collect::<String>();
    if trimmed.chars().count() > 120 {
        preview.push_str("...");
    }

    format!("body: {}", preview)
}

fn format_body_suffix(body: &str) -> String {
    let preview = preview_response(body);
    if preview.is_empty() {
        String::new()
    } else {
        format!(" ({})", preview)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dingtalk_sign_matches_reference() {
        let sign = dingtalk_sign("SECabcdef", "1700000000000").unwrap();
        assert_eq!(sign, "JISGksjyDysS1LOaD/BssnKBVXedPwjJ/s/DyTHUKMQ=");
    }

    #[test]
    fn dingtalk_requires_credential_and_secret() {
        let mut config = SmsPushConfig {
            provider: SmsPushProvider::Dingtalk,
            ..Default::default()
        };
        assert!(validate_config(&config).is_err());

        config.credential = "token".to_string();
        assert!(validate_config(&config).is_ok());

        config.sign_enabled = true;
        assert!(validate_config(&config).is_err());

        config.secret = "SECabcdef".to_string();
        assert!(validate_config(&config).is_ok());
    }
}
