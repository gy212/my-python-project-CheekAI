// AI Provider Service
// Implements GLM and Deepseek API calls

use regex::Regex;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::env;
use std::time::Instant;
use thiserror::Error;

const GLM_DEFAULT_URL: &str = "https://open.bigmodel.cn/api/paas/v4/chat/completions";
const DEEPSEEK_DEFAULT_URL: &str = "https://api.deepseek.com/chat/completions";
const ANTHROPIC_DEFAULT_URL: &str = "https://crs.itssx.com/api/v1/messages";
const OPENAI_RESPONSES_URL: &str = "https://ai.itssx.com/openai/responses";
const GEMINI_DEFAULT_URL: &str = "https://ai.itssx.com/api/v1/chat/completions";

pub const OPENAI_DEFAULT_MODEL: &str = "gpt-5.2";
pub const OPENAI_DEFAULT_MAX_OUTPUT_TOKENS: i32 = 8192;

fn clean_api_key(raw: &str) -> String {
    let mut s = raw.trim();

    // Trim wrapping quotes often introduced by copy/paste or JSON.
    if (s.starts_with('"') && s.ends_with('"')) || (s.starts_with('\'') && s.ends_with('\'')) {
        if s.len() >= 2 {
            s = &s[1..s.len() - 1];
        }
    }

    // Users sometimes paste full `Authorization: Bearer ...` values.
    let lower = s.to_ascii_lowercase();
    if lower.starts_with("bearer ") {
        // Slice the original by byte index 7.
        s = s.get(7..).unwrap_or(s);
    }

    s.trim().to_string()
}

fn is_official_anthropic_url(url: &str) -> bool {
    // Keep this intentionally simple to avoid pulling in a URL parsing dependency.
    url.contains("api.anthropic.com")
}

#[derive(Error, Debug)]
pub enum ProviderError {
    #[error("HTTP request failed: {0}")]
    HttpError(#[from] reqwest::Error),
    #[error("API error: {status} - {message}")]
    ApiError { status: u16, message: String },
    #[error("Missing content in response")]
    MissingContent,
    #[error("JSON parse error: {0}")]
    JsonError(String),
    #[error("API key not configured")]
    MissingApiKey,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderSpec {
    pub name: String,
    pub model: String,
}

pub fn parse_provider(spec: &str) -> ProviderSpec {
    let parts: Vec<&str> = spec.splitn(2, ':').collect();
    if parts.len() == 2 {
        ProviderSpec {
            name: parts[0].to_string(),
            model: parts[1].to_string(),
        }
    } else {
        ProviderSpec {
            name: spec.to_string(),
            model: String::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
struct ChatMessage {
    role: String,
    content: String,
}

#[derive(Debug, Clone, Serialize)]
struct ChatRequest {
    model: String,
    messages: Vec<ChatMessage>,
    max_tokens: i32,
    temperature: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    response_format: Option<ResponseFormat>,
    #[serde(skip_serializing_if = "Option::is_none")]
    reasoning: Option<ReasoningConfig>,
}

#[derive(Debug, Clone, Serialize)]
struct ResponseFormat {
    r#type: String,
}

#[derive(Debug, Clone, Serialize)]
struct ReasoningConfig {
    effort: String,
}

#[derive(Debug, Clone, Deserialize)]
struct ChatResponse {
    choices: Option<Vec<ChatChoice>>,
    reasoning_content: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
struct ChatChoice {
    message: Option<ChatMessageResponse>,
}

#[derive(Debug, Clone, Deserialize)]
struct ChatMessageResponse {
    content: Option<String>,
    reasoning_content: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatResult {
    pub content: String,
    pub latency_ms: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reasoning: Option<String>,
}

pub struct ProviderClient {
    client: Client,
    glm_url: String,
    deepseek_url: String,
    anthropic_url: String,
    openai_responses_url: String,
    gemini_url: String,
}

impl Default for ProviderClient {
    fn default() -> Self {
        Self::new()
    }
}

impl ProviderClient {
    pub fn new() -> Self {
        let client = Client::builder()
            .timeout(std::time::Duration::from_secs(180))
            .build()
            .unwrap_or_default();

        let glm_url = env::var("GLM_API_URL").unwrap_or_else(|_| GLM_DEFAULT_URL.to_string());
        let deepseek_url =
            env::var("DEEPSEEK_API_URL").unwrap_or_else(|_| DEEPSEEK_DEFAULT_URL.to_string());
        let anthropic_url =
            env::var("ANTHROPIC_API_URL").unwrap_or_else(|_| ANTHROPIC_DEFAULT_URL.to_string());
        let openai_responses_url = OPENAI_RESPONSES_URL.to_string();
        let gemini_url =
            env::var("GEMINI_API_URL").unwrap_or_else(|_| GEMINI_DEFAULT_URL.to_string());

        Self {
            client,
            glm_url,
            deepseek_url,
            anthropic_url,
            openai_responses_url,
            gemini_url,
        }
    }

    pub fn with_proxy(proxy_url: &str) -> Result<Self, ProviderError> {
        let proxy = reqwest::Proxy::all(proxy_url)?;
        let client = Client::builder()
            .timeout(std::time::Duration::from_secs(180))
            .proxy(proxy)
            .build()?;

        let glm_url = env::var("GLM_API_URL").unwrap_or_else(|_| GLM_DEFAULT_URL.to_string());
        let deepseek_url =
            env::var("DEEPSEEK_API_URL").unwrap_or_else(|_| DEEPSEEK_DEFAULT_URL.to_string());
        let anthropic_url =
            env::var("ANTHROPIC_API_URL").unwrap_or_else(|_| ANTHROPIC_DEFAULT_URL.to_string());
        let openai_responses_url = OPENAI_RESPONSES_URL.to_string();
        let gemini_url =
            env::var("GEMINI_API_URL").unwrap_or_else(|_| GEMINI_DEFAULT_URL.to_string());

        Ok(Self {
            client,
            glm_url,
            deepseek_url,
            anthropic_url,
            openai_responses_url,
            gemini_url,
        })
    }

    pub async fn call_glm(
        &self,
        model: &str,
        api_key: &str,
        system: &str,
        user: &str,
        max_tokens: i32,
        enable_reasoning: bool,
    ) -> Result<ChatResult, ProviderError> {
        self.call_glm_with_url(
            None,
            model,
            api_key,
            system,
            user,
            max_tokens,
            enable_reasoning,
        )
        .await
    }

    pub async fn call_glm_with_url(
        &self,
        custom_url: Option<&str>,
        model: &str,
        api_key: &str,
        system: &str,
        user: &str,
        max_tokens: i32,
        enable_reasoning: bool,
    ) -> Result<ChatResult, ProviderError> {
        let url = custom_url.unwrap_or(&self.glm_url);
        self.call_chat_api(
            url,
            model,
            api_key,
            system,
            user,
            max_tokens,
            enable_reasoning,
            true,
            true, // GLM supports json_object format
        )
        .await
    }

    pub async fn call_deepseek(
        &self,
        model: &str,
        api_key: &str,
        system: &str,
        user: &str,
        max_tokens: i32,
    ) -> Result<ChatResult, ProviderError> {
        self.call_deepseek_with_url(
            None,
            model,
            api_key,
            system,
            user,
            max_tokens,
        )
        .await
    }
    
    pub async fn call_deepseek_with_url(
        &self,
        custom_url: Option<&str>,
        model: &str,
        api_key: &str,
        system: &str,
        user: &str,
        max_tokens: i32,
    ) -> Result<ChatResult, ProviderError> {
        let url = custom_url.unwrap_or(&self.deepseek_url);
        self.call_chat_api(
            url,
            model,
            api_key,
            system,
            user,
            max_tokens,
            false,
            false,
            false, // DeepSeek: don't force json_object format unless prompt contains 'json'
        )
        .await
    }

    /// Call DeepSeek with JSON response format (prompt must contain 'json')
    pub async fn call_deepseek_json(
        &self,
        model: &str,
        api_key: &str,
        system: &str,
        user: &str,
        max_tokens: i32,
    ) -> Result<ChatResult, ProviderError> {
        self.call_deepseek_json_with_url(
            None,
            model,
            api_key,
            system,
            user,
            max_tokens,
        )
        .await
    }

    pub async fn call_deepseek_json_with_url(
        &self,
        custom_url: Option<&str>,
        model: &str,
        api_key: &str,
        system: &str,
        user: &str,
        max_tokens: i32,
    ) -> Result<ChatResult, ProviderError> {
        let url = custom_url.unwrap_or(&self.deepseek_url);
        self.call_chat_api(
            url,
            model,
            api_key,
            system,
            user,
            max_tokens,
            false,
            false,
            true, // Force JSON format
        )
        .await
    }

    pub async fn call_openai_responses(
        &self,
        model: &str,
        api_key: &str,
        input: &str,
    ) -> Result<ChatResult, ProviderError> {
        self.call_openai_responses_api(&self.openai_responses_url, model, api_key, input)
            .await
    }

    pub async fn call_openai_responses_with_url(
        &self,
        custom_url: Option<&str>,
        model: &str,
        api_key: &str,
        input: &str,
    ) -> Result<ChatResult, ProviderError> {
        let url = custom_url.unwrap_or(&self.openai_responses_url);
        self.call_openai_responses_api(url, model, api_key, input).await
    }

    pub async fn call_gemini(
        &self,
        model: &str,
        api_key: &str,
        system: &str,
        user: &str,
        max_tokens: i32,
    ) -> Result<ChatResult, ProviderError> {
        self.call_gemini_api(&self.gemini_url, model, api_key, system, user, max_tokens)
            .await
    }

    pub async fn call_gemini_with_url(
        &self,
        custom_url: Option<&str>,
        model: &str,
        api_key: &str,
        system: &str,
        user: &str,
        max_tokens: i32,
    ) -> Result<ChatResult, ProviderError> {
        let url = custom_url.unwrap_or(&self.gemini_url);
        self.call_gemini_api(url, model, api_key, system, user, max_tokens)
            .await
    }

    pub async fn call_anthropic(
        &self,
        model: &str,
        api_key: &str,
        system: &str,
        user: &str,
        max_tokens: i32,
    ) -> Result<ChatResult, ProviderError> {
        self.call_anthropic_api(&self.anthropic_url, model, api_key, system, user, max_tokens)
            .await
    }

    pub async fn call_anthropic_with_url(
        &self,
        custom_url: Option<&str>,
        model: &str,
        api_key: &str,
        system: &str,
        user: &str,
        max_tokens: i32,
    ) -> Result<ChatResult, ProviderError> {
        let url = custom_url.unwrap_or(&self.anthropic_url);
        self.call_anthropic_api(url, model, api_key, system, user, max_tokens)
            .await
    }

    async fn call_anthropic_api(
        &self,
        url: &str,
        model: &str,
        api_key: &str,
        system: &str,
        user: &str,
        max_tokens: i32,
    ) -> Result<ChatResult, ProviderError> {
        #[derive(Serialize)]
        struct AnthropicRequest {
            model: String,
            max_tokens: i32,
            messages: Vec<ChatMessage>,
        }

        #[derive(Deserialize)]
        struct AnthropicResponse {
            content: Option<Vec<AnthropicContent>>,
        }

        #[derive(Deserialize)]
        struct AnthropicContent {
            text: Option<String>,
        }

        // Combine system and user into a single user message (proxy doesn't support system field)
        let combined_content = if system.is_empty() {
            user.to_string()
        } else {
            format!("{}\n\n{}", system, user)
        };

        let request = AnthropicRequest {
            model: model.to_string(),
            max_tokens,
            messages: vec![ChatMessage {
                role: "user".to_string(),
                content: combined_content,
            }],
        };

        let start = Instant::now();
        let api_key = clean_api_key(api_key);

        let mut req = self
            .client
            .post(url)
            .header("Content-Type", "application/json")
            .json(&request);

        // Official Anthropic requires `x-api-key`. Many relays (including the default itssx
        // endpoint in this repo) expect `Authorization: Bearer ...`.
        if is_official_anthropic_url(url) {
            req = req
                .header("x-api-key", &api_key)
                .header("anthropic-version", "2024-10-22");
        } else {
            req = req
                .header("Authorization", format!("Bearer {}", api_key))
                .header("x-api-key", &api_key)
                .header("anthropic-version", "2024-10-22");
        }

        let response = req.send().await?;

        let latency_ms = start.elapsed().as_millis() as i64;
        let status = response.status();

        if !status.is_success() {
            let body = response.text().await.unwrap_or_default();
            return Err(ProviderError::ApiError {
                status: status.as_u16(),
                message: body,
            });
        }

        let data: AnthropicResponse = response
            .json()
            .await
            .map_err(|e| ProviderError::JsonError(e.to_string()))?;

        let content = data
            .content
            .and_then(|c| c.into_iter().next())
            .and_then(|c| c.text)
            .ok_or(ProviderError::MissingContent)?;

        Ok(ChatResult {
            content,
            latency_ms,
            reasoning: None,
        })
    }

    async fn call_openai_responses_api(
        &self,
        url: &str,
        model: &str,
        api_key: &str,
        input: &str,
    ) -> Result<ChatResult, ProviderError> {
        fn extract_first_json_object(s: &str) -> Option<&str> {
            let bytes = s.as_bytes();
            let mut i = 0usize;
            while i < bytes.len() && bytes[i].is_ascii_whitespace() {
                i += 1;
            }
            if i >= bytes.len() || bytes[i] != b'{' {
                return None;
            }

            let mut depth: i32 = 0;
            let mut in_string = false;
            let mut escape = false;
            for (idx, &b) in bytes.iter().enumerate().skip(i) {
                if in_string {
                    if escape {
                        escape = false;
                        continue;
                    }
                    if b == b'\\' {
                        escape = true;
                        continue;
                    }
                    if b == b'"' {
                        in_string = false;
                    }
                    continue;
                }

                match b {
                    b'"' => in_string = true,
                    b'{' => depth += 1,
                    b'}' => {
                        depth -= 1;
                        if depth == 0 {
                            return s.get(i..=idx);
                        }
                    }
                    _ => {}
                }
            }
            None
        }

        // Some relay providers advertise a "Responses API" endpoint but accept only the structured
        // `input` format (array of role/content blocks). Use that form for best compatibility.
        // Be generous with output token budget; gateways may accept either `max_output_tokens`
        // (Responses API) or `max_tokens` (Chat Completions style).
        let request = serde_json::json!({
            "model": model,
            "max_output_tokens": OPENAI_DEFAULT_MAX_OUTPUT_TOKENS,
            "max_tokens": OPENAI_DEFAULT_MAX_OUTPUT_TOKENS,
            "input": [{
                "role": "user",
                "content": [{ "type": "input_text", "text": input }]
            }]
        });

        let start = Instant::now();
        let api_key = clean_api_key(api_key);

        let response = self
            .client
            .post(url)
            .header("Authorization", format!("Bearer {}", api_key))
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .await?;

        let latency_ms = start.elapsed().as_millis() as i64;
        let status = response.status();

        if !status.is_success() {
            let body = response.text().await.unwrap_or_default();
            return Err(ProviderError::ApiError {
                status: status.as_u16(),
                message: body,
            });
        }

        // OpenAI Responses API format: {"output": [{"type": "message", "content": [{"type": "output_text", "text": "..."}]}]}
        let mut data: serde_json::Value = response
            .json()
            .await
            .map_err(|e| ProviderError::JsonError(e.to_string()))?;

        // Some relays return a JSON string that itself contains the JSON object, sometimes with
        // extra SSE lines appended (e.g. `event: error ...`). Parse the first JSON object only.
        if let Some(s) = data.as_str() {
            if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(s) {
                data = parsed;
            } else if let Some(json_obj) = extract_first_json_object(s) {
                if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(json_obj) {
                    data = parsed;
                }
            }
        }

        let content = data
            .get("output")
            .and_then(|o| o.as_array())
            .and_then(|o| o.first())
            .and_then(|msg| msg.get("content"))
            .and_then(|c| c.as_array())
            .and_then(|c| c.iter().find_map(|part| {
                part.get("text")
                    .and_then(|t| t.as_str())
                    .filter(|t| !t.trim().is_empty())
            }))
            // OpenAI Chat Completions style fallback
            .or_else(|| {
                data.get("choices")
                    .and_then(|c| c.as_array())
                    .and_then(|c| c.first())
                    .and_then(|c| c.get("message"))
                    .and_then(|m| m.get("content"))
                    .and_then(|t| t.as_str())
                    .filter(|t| !t.trim().is_empty())
            })
            .map(|s| s.to_string())
            .ok_or(ProviderError::MissingContent)?;

        Ok(ChatResult {
            content,
            latency_ms,
            reasoning: None,
        })
    }

    async fn call_gemini_api(
        &self,
        url: &str,
        model: &str,
        api_key: &str,
        system: &str,
        user: &str,
        max_tokens: i32,
    ) -> Result<ChatResult, ProviderError> {
        // Gemini uses OpenAI-compatible request but different response format
        let combined_content = if system.is_empty() {
            user.to_string()
        } else {
            format!("{}\n\n{}", system, user)
        };

        let request = serde_json::json!({
            "model": model,
            "messages": [{"role": "user", "content": combined_content}],
            "max_tokens": max_tokens
        });

        let start = Instant::now();
        let api_key = clean_api_key(api_key);

        let response = self
            .client
            .post(url)
            .header("Authorization", format!("Bearer {}", api_key))
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .await?;

        let latency_ms = start.elapsed().as_millis() as i64;
        let status = response.status();

        if !status.is_success() {
            let body = response.text().await.unwrap_or_default();
            return Err(ProviderError::ApiError {
                status: status.as_u16(),
                message: body,
            });
        }

        // Gemini response format: {"response":{"candidates":[{"content":{"parts":[{"text":"..."}]}}]}}
        let data: serde_json::Value = response
            .json()
            .await
            .map_err(|e| ProviderError::JsonError(e.to_string()))?;

        let content = data
            .get("response")
            .and_then(|r| r.get("candidates"))
            .and_then(|c| c.as_array())
            .and_then(|c| c.first())
            .and_then(|c| c.get("content"))
            .and_then(|content| {
                content
                    .get("parts")
                    .and_then(|p| p.as_array())
                    .and_then(|parts| {
                        parts.iter().find_map(|part| {
                            part.get("text")
                                .and_then(|t| t.as_str())
                                .filter(|t| !t.trim().is_empty())
                        })
                    })
                    .or_else(|| {
                        content
                            .get("text")
                            .and_then(|t| t.as_str())
                            .filter(|t| !t.trim().is_empty())
                    })
            })
            // Some gateways return an OpenAI-compatible payload instead of Gemini's `response.*`.
            .or_else(|| {
                data.get("choices")
                    .and_then(|c| c.as_array())
                    .and_then(|c| c.first())
                    .and_then(|c| c.get("message"))
                    .and_then(|m| m.get("content"))
                    .and_then(|t| t.as_str())
                    .filter(|t| !t.trim().is_empty())
            })
            .map(|s| s.to_string())
            .ok_or(ProviderError::MissingContent)?;

        Ok(ChatResult {
            content,
            latency_ms,
            reasoning: None,
        })
    }

    async fn call_chat_api(
        &self,
        url: &str,
        model: &str,
        api_key: &str,
        system: &str,
        user: &str,
        max_tokens: i32,
        enable_reasoning: bool,
        retry_on_empty: bool,
        use_json_format: bool,
    ) -> Result<ChatResult, ProviderError> {
        let mut request = ChatRequest {
            model: model.to_string(),
            messages: vec![
                ChatMessage {
                    role: "system".to_string(),
                    content: system.to_string(),
                },
                ChatMessage {
                    role: "user".to_string(),
                    content: user.to_string(),
                },
            ],
            max_tokens,
            temperature: 0.0,
            response_format: if use_json_format {
                Some(ResponseFormat {
                    r#type: "json_object".to_string(),
                })
            } else {
                None
            },
            reasoning: None,
        };

        if enable_reasoning {
            request.reasoning = Some(ReasoningConfig {
                effort: "high".to_string(),
            });
        }

        let start = Instant::now();
        let api_key_clean = clean_api_key(api_key);

        let response = self
            .client
            .post(url)
            .header("Authorization", format!("Bearer {}", api_key_clean))
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .await?;

        let latency_ms = start.elapsed().as_millis() as i64;
        let status = response.status();

        if !status.is_success() {
            let body = response.text().await.unwrap_or_default();
            return Err(ProviderError::ApiError {
                status: status.as_u16(),
                message: body,
            });
        }

        let data: ChatResponse = response
            .json()
            .await
            .map_err(|e| ProviderError::JsonError(e.to_string()))?;

        // Extract content
        let mut content = data
            .choices
            .as_ref()
            .and_then(|c| c.first())
            .and_then(|c| c.message.as_ref())
            .and_then(|m| m.content.clone());

        // Extract reasoning
        let reasoning = data
            .choices
            .as_ref()
            .and_then(|c| c.first())
            .and_then(|c| c.message.as_ref())
            .and_then(|m| m.reasoning_content.clone())
            .or(data.reasoning_content);

        // Try to extract JSON from reasoning if content is empty
        if content.is_none() {
            if let Some(ref r) = reasoning {
                let json_re = Regex::new(r"\{.*\}").unwrap();
                if let Some(m) = json_re.find(r) {
                    content = Some(m.as_str().to_string());
                }
            }
        }

        // Retry without reasoning if content is still empty
        if content.is_none() && retry_on_empty && enable_reasoning {
            return Box::pin(self.call_chat_api(
                url, model, api_key, system, user, max_tokens, false, false, use_json_format,
            ))
            .await;
        }

        let content = content.ok_or(ProviderError::MissingContent)?;

        Ok(ChatResult {
            content,
            latency_ms,
            reasoning,
        })
    }
}

/// Get API key from environment or config file
pub fn get_api_key(provider: &str) -> Option<String> {
    // Try environment variables first
    let env_keys = match provider {
        "glm" => vec!["GLM_API_KEY", "CHEEKAI_GLM_API_KEY"],
        "deepseek" => vec!["DEEPSEEK_API_KEY", "CHEEKAI_DEEPSEEK_API_KEY"],
        "anthropic" | "claude" => vec!["ANTHROPIC_API_KEY", "CHEEKAI_ANTHROPIC_API_KEY"],
        "openai" => vec!["OPENAI_API_KEY", "CHEEKAI_OPENAI_API_KEY"],
        "gemini" => vec!["GEMINI_API_KEY", "CHEEKAI_GEMINI_API_KEY"],
        _ => vec![],
    };

    for key in env_keys {
        if let Ok(val) = env::var(key) {
            let v = clean_api_key(&val);
            if !v.is_empty() {
                return Some(v);
            }
        }
    }

    // Try config file
    if let Some(config_dir) = super::ConfigStore::default_config_dir() {
        let store = super::ConfigStore::new(config_dir);
        if let Ok(Some(key)) = store.get_api_key(provider) {
            let v = clean_api_key(&key);
            if !v.is_empty() {
                return Some(v);
            }
        }

        // Handle common aliases in config keys.
        if provider == "anthropic" {
            if let Ok(Some(key)) = store.get_api_key("claude") {
                let v = clean_api_key(&key);
                if !v.is_empty() {
                    return Some(v);
                }
            }
        } else if provider == "claude" {
            if let Ok(Some(key)) = store.get_api_key("anthropic") {
                let v = clean_api_key(&key);
                if !v.is_empty() {
                    return Some(v);
                }
            }
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_provider() {
        let spec = parse_provider("glm:glm-4-plus");
        assert_eq!(spec.name, "glm");
        assert_eq!(spec.model, "glm-4-plus");

        let spec2 = parse_provider("deepseek");
        assert_eq!(spec2.name, "deepseek");
        assert_eq!(spec2.model, "");
    }

    #[test]
    fn test_provider_client_creation() {
        let client = ProviderClient::new();
        assert!(client.glm_url.contains("bigmodel.cn"));
    }

    #[test]
    fn test_clean_api_key() {
        assert_eq!(clean_api_key("  sk-xxx  "), "sk-xxx");
        assert_eq!(clean_api_key("Bearer sk-xxx"), "sk-xxx");
        assert_eq!(clean_api_key("bearer sk-xxx"), "sk-xxx");
        assert_eq!(clean_api_key("\"sk-xxx\""), "sk-xxx");
        assert_eq!(clean_api_key("'sk-xxx'"), "sk-xxx");
    }
}
