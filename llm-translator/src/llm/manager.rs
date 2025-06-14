use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::{Mutex, RwLock, mpsc};
use tokio::time::interval;
use tracing::{debug, error, info, warn};
use dashmap::DashMap;
use tokio_util::sync::CancellationToken;

use super::{LlmClient, LlmError, ChatCompletionRequest, ChatMessage, TextSplitter, TranslationChunk, TranslatedChunk};
use crate::validation::{RateLimiter, RateLimitError};
use crate::config::Config;

#[derive(Debug, Clone)]
pub struct TranslationRequest {
    pub id: String,
    pub text: String,
    pub prompt_preset: String,
    pub priority: RequestPriority,
    pub created_at: Instant,
    pub cancellation_token: CancellationToken,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum RequestPriority {
    Low = 0,
    Normal = 1,
    High = 2,
}

#[derive(Debug, Clone)]
pub struct TranslationResult {
    pub request_id: String,
    pub original_text: String,
    pub translated_text: String,
    pub tokens_used: u32,
    pub duration: Duration,
}

#[derive(Debug)]
pub enum TranslationEvent {
    Completed(TranslationResult),
    Failed { request_id: String, error: String },
    Cancelled { request_id: String },
    RateLimited { request_id: String, wait_time: Duration },
}

pub struct TranslationManager {
    client: Arc<LlmClient>,
    config: Arc<RwLock<Config>>,
    rate_limiter: Arc<RateLimiter>,
    
    // Queue management
    queue: Arc<Mutex<VecDeque<TranslationRequest>>>,
    active_requests: Arc<DashMap<String, TranslationRequest>>,
    
    // Cache for recent translations
    cache: Arc<DashMap<String, (String, Instant)>>,
    cache_ttl: Duration,
    
    // Deduplication
    pending_hashes: Arc<DashMap<u64, Vec<String>>>, // hash -> request_ids
    
    // Event channel
    event_sender: mpsc::Sender<TranslationEvent>,
}

impl TranslationManager {
    pub fn new(
        client: LlmClient,
        config: Arc<RwLock<Config>>,
        rate_limiter: RateLimiter,
        event_sender: mpsc::Sender<TranslationEvent>,
    ) -> Self {
        Self {
            client: Arc::new(client),
            config,
            rate_limiter: Arc::new(rate_limiter),
            queue: Arc::new(Mutex::new(VecDeque::new())),
            active_requests: Arc::new(DashMap::new()),
            cache: Arc::new(DashMap::new()),
            cache_ttl: Duration::from_secs(300), // 5 minutes
            pending_hashes: Arc::new(DashMap::new()),
            event_sender,
        }
    }
    
    /// Submit a translation request
    pub async fn translate(
        &self,
        text: String,
        prompt_preset: String,
        priority: RequestPriority,
    ) -> Result<String, String> {
        let request_id = self.generate_request_id();
        
        // Check cache first
        let cache_key = self.cache_key(&text, &prompt_preset);
        if let Some(cached) = self.get_cached(&cache_key).await {
            debug!("Translation found in cache");
            return Ok(cached);
        }
        
        // Check for duplicate pending requests
        let text_hash = self.hash_text(&text);
        if let Some(mut pending_ids) = self.pending_hashes.get_mut(&text_hash) {
            debug!("Found pending request with same text, deduplicating");
            pending_ids.push(request_id.clone());
            // Wait for the original request to complete
            // In real implementation, we'd use a more sophisticated notification system
            return Err("Request deduplicated".to_string());
        }
        
        // Create new request
        let request = TranslationRequest {
            id: request_id.clone(),
            text,
            prompt_preset,
            priority,
            created_at: Instant::now(),
            cancellation_token: CancellationToken::new(),
        };
        
        // Add to deduplication tracking
        self.pending_hashes.insert(text_hash, vec![request_id.clone()]);
        
        // Add to queue
        self.enqueue_request(request).await;
        
        Ok(request_id)
    }
    
    /// Cancel a translation request
    pub async fn cancel(&self, request_id: &str) -> bool {
        // Check if in queue
        let mut queue = self.queue.lock().await;
        if let Some(pos) = queue.iter().position(|r| r.id == request_id) {
            let request = queue.remove(pos).unwrap();
            request.cancellation_token.cancel();
            drop(queue);
            
            self.send_event(TranslationEvent::Cancelled {
                request_id: request_id.to_string(),
            }).await;
            return true;
        }
        drop(queue);
        
        // Check if active
        if let Some((_, request)) = self.active_requests.remove(request_id) {
            request.cancellation_token.cancel();
            self.send_event(TranslationEvent::Cancelled {
                request_id: request_id.to_string(),
            }).await;
            return true;
        }
        
        false
    }
    
    /// Start the translation processing loop
    pub async fn start(self: Arc<Self>) {
        info!("Translation manager started");
        
        // Start cache cleanup task
        let manager = Arc::clone(&self);
        tokio::spawn(async move {
            manager.cache_cleanup_loop().await;
        });
        
        // Main processing loop
        let mut ticker = interval(Duration::from_millis(100));
        
        loop {
            ticker.tick().await;
            
            // Process queue
            if let Some(request) = self.get_next_request().await {
                let manager = Arc::clone(&self);
                tokio::spawn(async move {
                    manager.process_request(request).await;
                });
            }
        }
    }
    
    async fn enqueue_request(&self, request: TranslationRequest) {
        let mut queue = self.queue.lock().await;
        
        // Insert based on priority (higher priority first)
        let insert_pos = queue.iter().position(|r| r.priority < request.priority)
            .unwrap_or(queue.len());
            
        queue.insert(insert_pos, request);
        
        debug!("Request enqueued, queue size: {}", queue.len());
    }
    
    async fn get_next_request(&self) -> Option<TranslationRequest> {
        // Check rate limit
        if let Err(e) = self.rate_limiter.check_and_update() {
            match e {
                RateLimitError::MinuteLimit { wait_time } => {
                    debug!("Rate limited, waiting {:?}", wait_time);
                    return None;
                }
                RateLimitError::DailyLimit { .. } => {
                    warn!("Daily limit reached");
                    return None;
                }
            }
        }
        
        let mut queue = self.queue.lock().await;
        queue.pop_front()
    }
    
    async fn process_request(&self, request: TranslationRequest) {
        let request_id = request.id.clone();
        let text_hash = self.hash_text(&request.text);
        
        // Mark as active
        self.active_requests.insert(request_id.clone(), request.clone());
        
        // Build API request
        let api_request = match self.build_api_request(&request).await {
            Ok(req) => req,
            Err(e) => {
                error!("Failed to build API request: {}", e);
                self.handle_request_failure(&request_id, e).await;
                return;
            }
        };
        
        // Send request
        let start_time = Instant::now();
        match self.client.chat_completion(api_request, Some(request.cancellation_token)).await {
            Ok(response) => {
                if let Some(translated) = response.get_content() {
                    let duration = start_time.elapsed();
                    
                    // Cache the result
                    let cache_key = self.cache_key(&request.text, &request.prompt_preset);
                    self.cache.insert(cache_key, (translated.to_string(), Instant::now()));
                    
                    // Create result
                    let result = TranslationResult {
                        request_id: request_id.clone(),
                        original_text: request.text,
                        translated_text: translated.to_string(),
                        tokens_used: response.usage.total_tokens,
                        duration,
                    };
                    
                    // Send completion event
                    self.send_event(TranslationEvent::Completed(result)).await;
                    
                    // Notify deduplicated requests
                    if let Some((_, pending_ids)) = self.pending_hashes.remove(&text_hash) {
                        for id in pending_ids {
                            if id != request_id {
                                // In real implementation, notify these requests
                                debug!("Notifying deduplicated request: {}", id);
                            }
                        }
                    }
                } else {
                    self.handle_request_failure(
                        &request_id,
                        "No content in response".to_string()
                    ).await;
                }
            }
            Err(e) => {
                match &e {
                    LlmError::RateLimited { retry_after } => {
                        // Re-queue the request
                        self.active_requests.remove(&request_id);
                        self.enqueue_request(request).await;
                        
                        self.send_event(TranslationEvent::RateLimited {
                            request_id,
                            wait_time: retry_after.unwrap_or(Duration::from_secs(60)),
                        }).await;
                    }
                    LlmError::RequestTooLarge(tokens) => {
                        // Try to split the text
                        warn!("Request too large: {} tokens", tokens);
                        self.handle_request_failure(&request_id, e.to_string()).await;
                    }
                    _ => {
                        self.handle_request_failure(&request_id, e.to_string()).await;
                    }
                }
            }
        }
        
        // Clean up
        self.active_requests.remove(&request_id);
        self.pending_hashes.remove(&text_hash);
    }
    
    async fn build_api_request(&self, request: &TranslationRequest) -> Result<ChatCompletionRequest, String> {
        let config = self.config.read().await;
        
        // Get prompt preset
        let prompt = config.prompt.presets.get(&request.prompt_preset)
            .or_else(|| config.prompt.custom.get(&request.prompt_preset))
            .ok_or_else(|| format!("Prompt preset '{}' not found", request.prompt_preset))?;
        
        Ok(ChatCompletionRequest::new(&config.api.model)
            .with_message(ChatMessage::system(&prompt.system))
            .with_message(ChatMessage::user(&request.text))
            .with_temperature(config.api.temperature)
            .with_user_id(&request.id))
    }
    
    async fn handle_request_failure(&self, request_id: &str, error: String) {
        error!("Request {} failed: {}", request_id, error);
        
        self.active_requests.remove(request_id);
        
        self.send_event(TranslationEvent::Failed {
            request_id: request_id.to_string(),
            error,
        }).await;
    }
    
    async fn send_event(&self, event: TranslationEvent) {
        if let Err(e) = self.event_sender.send(event).await {
            error!("Failed to send translation event: {}", e);
        }
    }
    
    async fn get_cached(&self, key: &str) -> Option<String> {
        self.cache.get(key)
            .filter(|(_, created)| created.elapsed() < self.cache_ttl)
            .map(|entry| entry.0.clone())
    }
    
    async fn cache_cleanup_loop(&self) {
        let mut ticker = interval(Duration::from_secs(60));
        
        loop {
            ticker.tick().await;
            
            let now = Instant::now();
            self.cache.retain(|_, (_, created)| {
                now.duration_since(*created) < self.cache_ttl
            });
            
            debug!("Cache cleanup: {} entries remaining", self.cache.len());
        }
    }
    
    fn generate_request_id(&self) -> String {
        format!("req_{}", uuid::Uuid::new_v4())
    }
    
    fn cache_key(&self, text: &str, preset: &str) -> String {
        format!("{}:{}", preset, self.hash_text(text))
    }
    
    fn hash_text(&self, text: &str) -> u64 {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        
        let mut hasher = DefaultHasher::new();
        text.hash(&mut hasher);
        hasher.finish()
    }
    
    /// Get queue statistics
    pub async fn get_stats(&self) -> QueueStats {
        let queue = self.queue.lock().await;
        
        QueueStats {
            queued: queue.len(),
            active: self.active_requests.len(),
            cached: self.cache.len(),
            rate_limit_remaining_minute: self.rate_limiter.remaining_this_minute(),
            rate_limit_remaining_day: self.rate_limiter.remaining_today(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct QueueStats {
    pub queued: usize,
    pub active: usize,
    pub cached: usize,
    pub rate_limit_remaining_minute: usize,
    pub rate_limit_remaining_day: usize,
}

impl Default for RequestPriority {
    fn default() -> Self {
        Self::Normal
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;
    use crate::validation::RateLimiter;

    #[tokio::test]
    async fn test_request_priority_ordering() {
        let (tx, _rx) = mpsc::channel(10);
        let config = Arc::new(RwLock::new(Config::default()));
        let client = LlmClient::new(&config.read().await.api, "test".to_string()).unwrap();
        let rate_limiter = RateLimiter::new(10, 100);
        
        let manager = TranslationManager::new(client, config, rate_limiter, tx);
        
        // Add requests with different priorities
        manager.enqueue_request(TranslationRequest {
            id: "1".to_string(),
            text: "test".to_string(),
            prompt_preset: "general".to_string(),
            priority: RequestPriority::Low,
            created_at: Instant::now(),
            cancellation_token: CancellationToken::new(),
        }).await;
        
        manager.enqueue_request(TranslationRequest {
            id: "2".to_string(),
            text: "test".to_string(),
            prompt_preset: "general".to_string(),
            priority: RequestPriority::High,
            created_at: Instant::now(),
            cancellation_token: CancellationToken::new(),
        }).await;
        
        manager.enqueue_request(TranslationRequest {
            id: "3".to_string(),
            text: "test".to_string(),
            prompt_preset: "general".to_string(),
            priority: RequestPriority::Normal,
            created_at: Instant::now(),
            cancellation_token: CancellationToken::new(),
        }).await;
        
        // High priority should be first
        let next = manager.get_next_request().await.unwrap();
        assert_eq!(next.id, "2");
        
        // Normal priority should be second
        let next = manager.get_next_request().await.unwrap();
        assert_eq!(next.id, "3");
        
        // Low priority should be last
        let next = manager.get_next_request().await.unwrap();
        assert_eq!(next.id, "1");
    }
    
    #[test]
    fn test_cache_key_generation() {
        let (tx, _rx) = mpsc::channel(10);
        let config = Arc::new(RwLock::new(Config::default()));
        let client = LlmClient::new(&Config::default().api, "test".to_string()).unwrap();
        let rate_limiter = RateLimiter::new(10, 100);
        
        let manager = TranslationManager::new(client, config, rate_limiter, tx);
        
        let key1 = manager.cache_key("Hello world", "general");
        let key2 = manager.cache_key("Hello world", "formal");
        let key3 = manager.cache_key("Different text", "general");
        
        // Same text, different preset = different keys
        assert_ne!(key1, key2);
        
        // Different text, same preset = different keys
        assert_ne!(key1, key3);
        
        // Same inputs = same key
        let key4 = manager.cache_key("Hello world", "general");
        assert_eq!(key1, key4);
    }
}