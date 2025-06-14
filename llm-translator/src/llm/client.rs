use reqwest::{Client, StatusCode};
use std::time::Duration;
use thiserror::Error;
use tokio::time::sleep;
use tracing::{debug, error, info, warn};
use tokio_util::sync::CancellationToken;

use super::api_types::{ChatCompletionRequest, ChatCompletionResponse, ErrorResponse};
use crate::config::ApiSettings;

#[derive(Error, Debug)]
pub enum LlmError {
    #[error("HTTP request failed: {0}")]
    RequestError(#[from] reqwest::Error),
    
    #[error("API error: {message}")]
    ApiError { message: String, code: Option<String> },
    
    #[error("Rate limit exceeded, retry after {retry_after:?}")]
    RateLimited { retry_after: Option<Duration> },
    
    #[error("Request timeout after {0} seconds")]
    Timeout(u64),
    
    #[error("Request cancelled")]
    Cancelled,
    
    #[error("Invalid API key")]
    InvalidApiKey,
    
    #[error("Model not found: {0}")]
    ModelNotFound(String),
    
    #[error("Request too large: {0} tokens")]
    RequestTooLarge(u32),
    
    #[error("Insufficient quota")]
    InsufficientQuota,
    
    #[error("Service unavailable")]
    ServiceUnavailable,
    
    #[error("Deserialization error: {0}")]
    DeserializationError(String),
}

pub struct LlmClient {
    client: Client,
    api_key: String,
    endpoint: String,
    max_retries: u32,
    timeout: Duration,
}

impl LlmClient {
    pub fn new(settings: &ApiSettings, api_key: String) -> Result<Self, LlmError> {
        let client = Client::builder()
            .timeout(Duration::from_secs(settings.timeout_seconds))
            .build()?;
            
        Ok(Self {
            client,
            api_key,
            endpoint: settings.endpoint.clone(),
            max_retries: settings.max_retries,
            timeout: Duration::from_secs(settings.timeout_seconds),
        })
    }
    
    pub async fn chat_completion(
        &self,
        request: ChatCompletionRequest,
        cancellation_token: Option<CancellationToken>,
    ) -> Result<ChatCompletionResponse, LlmError> {
        let mut retry_count = 0;
        
        loop {
            if let Some(token) = &cancellation_token {
                if token.is_cancelled() {
                    return Err(LlmError::Cancelled);
                }
            }
            
            match self.send_request(&request).await {
                Ok(response) => return Ok(response),
                Err(err) => {
                    if !self.should_retry(&err, retry_count) {
                        return Err(err);
                    }
                    
                    retry_count += 1;
                    let delay = self.calculate_retry_delay(retry_count, &err);
                    
                    warn!(
                        "Request failed (attempt {}/{}): {}. Retrying in {:?}",
                        retry_count, self.max_retries, err, delay
                    );
                    
                    // Check cancellation before sleeping
                    if let Some(token) = &cancellation_token {
                        tokio::select! {
                            _ = sleep(delay) => {},
                            _ = token.cancelled() => return Err(LlmError::Cancelled),
                        }
                    } else {
                        sleep(delay).await;
                    }
                }
            }
        }
    }
    
    async fn send_request(
        &self,
        request: &ChatCompletionRequest,
    ) -> Result<ChatCompletionResponse, LlmError> {
        debug!("Sending request to OpenAI API");
        
        let response = self.client
            .post(&self.endpoint)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(request)
            .send()
            .await?;
            
        let status = response.status();
        
        if status.is_success() {
            let body = response.text().await?;
            serde_json::from_str::<ChatCompletionResponse>(&body)
                .map_err(|e| LlmError::DeserializationError(format!("{}: {}", e, body)))
        } else {
            self.handle_error_response(status, response).await
        }
    }
    
    async fn handle_error_response(
        &self,
        status: StatusCode,
        response: reqwest::Response,
    ) -> Result<ChatCompletionResponse, LlmError> {
        let body = response.text().await.unwrap_or_default();
        
        // Try to parse error response
        let error_detail = serde_json::from_str::<ErrorResponse>(&body)
            .map(|e| e.error)
            .ok();
            
        match status {
            StatusCode::UNAUTHORIZED => Err(LlmError::InvalidApiKey),
            StatusCode::NOT_FOUND => {
                if let Some(detail) = error_detail {
                    if detail.message.contains("model") {
                        Err(LlmError::ModelNotFound(detail.message))
                    } else {
                        Err(LlmError::ApiError {
                            message: detail.message,
                            code: detail.code,
                        })
                    }
                } else {
                    Err(LlmError::ApiError {
                        message: "Not found".to_string(),
                        code: None,
                    })
                }
            }
            StatusCode::TOO_MANY_REQUESTS => {
                // Try to parse Retry-After header
                let retry_after = response
                    .headers()
                    .get("Retry-After")
                    .and_then(|v| v.to_str().ok())
                    .and_then(|s| s.parse::<u64>().ok())
                    .map(Duration::from_secs);
                    
                Err(LlmError::RateLimited { retry_after })
            }
            StatusCode::PAYLOAD_TOO_LARGE => {
                if let Some(detail) = error_detail {
                    // Try to extract token count from error message
                    let tokens = detail.message
                        .split_whitespace()
                        .find_map(|w| w.parse::<u32>().ok())
                        .unwrap_or(0);
                    Err(LlmError::RequestTooLarge(tokens))
                } else {
                    Err(LlmError::RequestTooLarge(0))
                }
            }
            StatusCode::INSUFFICIENT_STORAGE => Err(LlmError::InsufficientQuota),
            StatusCode::SERVICE_UNAVAILABLE | StatusCode::BAD_GATEWAY => {
                Err(LlmError::ServiceUnavailable)
            }
            _ => {
                if let Some(detail) = error_detail {
                    Err(LlmError::ApiError {
                        message: detail.message,
                        code: detail.code,
                    })
                } else {
                    Err(LlmError::ApiError {
                        message: format!("HTTP {}: {}", status, body),
                        code: None,
                    })
                }
            }
        }
    }
    
    fn should_retry(&self, error: &LlmError, retry_count: u32) -> bool {
        if retry_count >= self.max_retries {
            return false;
        }
        
        matches!(
            error,
            LlmError::RequestError(_) |
            LlmError::RateLimited { .. } |
            LlmError::ServiceUnavailable |
            LlmError::Timeout(_)
        )
    }
    
    fn calculate_retry_delay(&self, retry_count: u32, error: &LlmError) -> Duration {
        match error {
            LlmError::RateLimited { retry_after: Some(duration) } => *duration,
            _ => {
                // Exponential backoff with jitter
                let base_delay = Duration::from_millis(100);
                let exponential = base_delay * 2u32.pow(retry_count - 1);
                let jitter = Duration::from_millis(fastrand::u64(0..100));
                exponential + jitter
            }
        }
    }
    
    // Helper method to check if API key is valid without making a real request
    pub async fn validate_api_key(&self) -> Result<(), LlmError> {
        let request = ChatCompletionRequest::new("gpt-3.5-turbo")
            .with_user_message("test")
            .with_max_tokens(1);
            
        match self.send_request(&request).await {
            Ok(_) => Ok(()),
            Err(LlmError::InvalidApiKey) => Err(LlmError::InvalidApiKey),
            Err(_) => Ok(()), // Other errors don't mean invalid key
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::ApiSettings;

    fn create_test_client() -> LlmClient {
        let settings = ApiSettings {
            endpoint: "https://api.openai.com/v1/chat/completions".to_string(),
            model: "gpt-4.1-nano".to_string(),
            temperature: 0.3,
            max_retries: 3,
            timeout_seconds: 30,
            api_key: None,
        };
        
        LlmClient::new(&settings, "test-key".to_string()).unwrap()
    }

    #[test]
    fn test_client_creation() {
        let client = create_test_client();
        assert_eq!(client.max_retries, 3);
        assert_eq!(client.timeout, Duration::from_secs(30));
    }

    #[test]
    fn test_should_retry() {
        let client = create_test_client();
        
        // Should retry on rate limit
        assert!(client.should_retry(&LlmError::RateLimited { retry_after: None }, 0));
        
        // Should retry on service unavailable
        assert!(client.should_retry(&LlmError::ServiceUnavailable, 0));
        
        // Should not retry on invalid API key
        assert!(!client.should_retry(&LlmError::InvalidApiKey, 0));
        
        // Should not retry after max retries
        assert!(!client.should_retry(&LlmError::ServiceUnavailable, 3));
    }

    #[test]
    fn test_retry_delay_calculation() {
        let client = create_test_client();
        
        // Rate limited with specific retry_after
        let delay = client.calculate_retry_delay(
            1,
            &LlmError::RateLimited { retry_after: Some(Duration::from_secs(5)) }
        );
        assert_eq!(delay, Duration::from_secs(5));
        
        // Exponential backoff for other errors
        let delay = client.calculate_retry_delay(2, &LlmError::ServiceUnavailable);
        // Should be around 200ms + jitter
        assert!(delay >= Duration::from_millis(200));
        assert!(delay < Duration::from_millis(300));
    }
}