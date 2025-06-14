use std::collections::VecDeque;
use std::time::{Duration, Instant};
use chrono::{DateTime, Utc, Datelike};
use thiserror::Error;
use std::sync::{Arc, Mutex};
use std::sync::atomic::{AtomicUsize, Ordering};

#[derive(Error, Debug)]
pub enum RateLimitError {
    #[error("Rate limit exceeded: please wait {wait_time:?}")]
    MinuteLimit { wait_time: Duration },
    
    #[error("Daily limit exceeded: {used}/{max} requests used today")]
    DailyLimit { used: usize, max: usize },
}

// Internal state that needs synchronization
#[derive(Debug)]
struct RateLimiterState {
    requests: VecDeque<Instant>,
    daily_count: usize,
    last_reset: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct RateLimiter {
    state: Arc<Mutex<RateLimiterState>>,
    max_per_minute: AtomicUsize,
    max_per_day: AtomicUsize,
}

impl RateLimiter {
    pub fn new(max_per_minute: usize, max_per_day: usize) -> Self {
        let state = RateLimiterState {
            requests: VecDeque::with_capacity(max_per_minute.min(1000)), // Cap capacity
            daily_count: 0,
            last_reset: Utc::now(),
        };
        
        Self {
            state: Arc::new(Mutex::new(state)),
            max_per_minute: AtomicUsize::new(max_per_minute),
            max_per_day: AtomicUsize::new(max_per_day),
        }
    }

    pub fn check_and_update(&self) -> Result<(), RateLimitError> {
        let now = Instant::now();
        let today = Utc::now();
        
        // Lock the state for the entire operation
        let mut state = self.state.lock()
            .map_err(|_| RateLimitError::DailyLimit { used: 0, max: 0 })?;
        
        // Get current limits
        let max_per_minute = self.max_per_minute.load(Ordering::Acquire);
        let max_per_day = self.max_per_day.load(Ordering::Acquire);
        
        // Reset daily counter if it's a new day
        if today.date_naive() != state.last_reset.date_naive() {
            state.daily_count = 0;
            state.last_reset = today;
            state.requests.clear(); // Also clear minute requests
        }
        
        // Remove requests older than 1 minute
        let one_minute_ago = now - Duration::from_secs(60);
        state.requests.retain(|&req_time| req_time > one_minute_ago);
        
        // Check minute limit
        if state.requests.len() >= max_per_minute {
            if let Some(&oldest) = state.requests.front() {
                let elapsed = now.duration_since(oldest);
                if elapsed < Duration::from_secs(60) {
                    let wait_time = Duration::from_secs(60) - elapsed;
                    return Err(RateLimitError::MinuteLimit { wait_time });
                }
            }
        }
        
        // Check daily limit
        if state.daily_count >= max_per_day {
            return Err(RateLimitError::DailyLimit {
                used: state.daily_count,
                max: max_per_day,
            });
        }
        
        // Update counters
        state.requests.push_back(now);
        state.daily_count += 1;
        
        // Prevent unbounded growth
        if state.requests.len() > max_per_minute * 2 {
            state.requests.drain(..state.requests.len() - max_per_minute);
        }
        
        Ok(())
    }

    pub fn remaining_today(&self) -> usize {
        if let Ok(state) = self.state.lock() {
            self.max_per_day.load(Ordering::Acquire).saturating_sub(state.daily_count)
        } else {
            0
        }
    }

    pub fn remaining_this_minute(&self) -> usize {
        if let Ok(state) = self.state.lock() {
            self.max_per_minute.load(Ordering::Acquire).saturating_sub(state.requests.len())
        } else {
            0
        }
    }

    pub fn next_available(&self) -> Option<Duration> {
        if let Ok(state) = self.state.lock() {
            let max_per_minute = self.max_per_minute.load(Ordering::Acquire);
            if state.requests.len() >= max_per_minute {
                if let Some(&oldest) = state.requests.front() {
                    let elapsed = Instant::now().duration_since(oldest);
                    if elapsed < Duration::from_secs(60) {
                        return Some(Duration::from_secs(60) - elapsed);
                    }
                }
            }
        }
        None
    }

    pub fn reset_daily_count(&self) {
        if let Ok(mut state) = self.state.lock() {
            state.daily_count = 0;
            state.last_reset = Utc::now();
            state.requests.clear();
        }
    }

    pub fn get_stats(&self) -> RateLimiterStats {
        if let Ok(state) = self.state.lock() {
            RateLimiterStats {
                requests_this_minute: state.requests.len(),
                requests_today: state.daily_count,
                max_per_minute: self.max_per_minute.load(Ordering::Acquire),
                max_per_day: self.max_per_day.load(Ordering::Acquire),
                last_reset: state.last_reset,
            }
        } else {
            RateLimiterStats {
                requests_this_minute: 0,
                requests_today: 0,
                max_per_minute: self.max_per_minute.load(Ordering::Acquire),
                max_per_day: self.max_per_day.load(Ordering::Acquire),
                last_reset: Utc::now(),
            }
        }
    }
    
    pub fn update_limits(&self, max_per_minute: usize, max_per_day: usize) {
        self.max_per_minute.store(max_per_minute, Ordering::Release);
        self.max_per_day.store(max_per_day, Ordering::Release);
    }
}

#[derive(Debug, Clone)]
pub struct RateLimiterStats {
    pub requests_this_minute: usize,
    pub requests_today: usize,
    pub max_per_minute: usize,
    pub max_per_day: usize,
    pub last_reset: DateTime<Utc>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;

    #[test]
    fn test_minute_rate_limit() {
        let limiter = RateLimiter::new(3, 100);
        
        // First 3 requests should succeed
        assert!(limiter.check_and_update().is_ok());
        assert!(limiter.check_and_update().is_ok());
        assert!(limiter.check_and_update().is_ok());
        
        // 4th request should fail
        assert!(matches!(
            limiter.check_and_update(),
            Err(RateLimitError::MinuteLimit { .. })
        ));
    }

    #[test]
    fn test_daily_rate_limit() {
        let limiter = RateLimiter::new(100, 5);
        
        // Use up daily limit
        for _ in 0..5 {
            assert!(limiter.check_and_update().is_ok());
        }
        
        // Next request should fail with daily limit
        assert!(matches!(
            limiter.check_and_update(),
            Err(RateLimitError::DailyLimit { .. })
        ));
    }

    #[test]
    fn test_minute_window_sliding() {
        let limiter = RateLimiter::new(2, 100);
        
        // Use up the limit
        assert!(limiter.check_and_update().is_ok());
        assert!(limiter.check_and_update().is_ok());
        assert!(limiter.check_and_update().is_err());
        
        // Reset for testing
        limiter.reset_daily_count();
        
        // Should work again
        assert!(limiter.check_and_update().is_ok());
    }

    #[test]
    fn test_remaining_counts() {
        let limiter = RateLimiter::new(5, 10);
        
        assert_eq!(limiter.remaining_today(), 10);
        assert_eq!(limiter.remaining_this_minute(), 5);
        
        limiter.check_and_update().unwrap();
        
        assert_eq!(limiter.remaining_today(), 9);
        assert_eq!(limiter.remaining_this_minute(), 4);
    }

    #[test]
    fn test_next_available() {
        let limiter = RateLimiter::new(1, 100);
        
        // First request succeeds
        assert!(limiter.check_and_update().is_ok());
        
        // Should have wait time
        let wait = limiter.next_available();
        assert!(wait.is_some());
        assert!(wait.unwrap() <= Duration::from_secs(60));
    }
    
    #[test]
    fn test_thread_safety() {
        use std::sync::Arc;
        use std::thread;
        
        let limiter = Arc::new(RateLimiter::new(10, 100));
        let mut handles = vec![];
        
        // Spawn multiple threads trying to use the rate limiter
        for _ in 0..5 {
            let limiter_clone = Arc::clone(&limiter);
            let handle = thread::spawn(move || {
                for _ in 0..3 {
                    let _ = limiter_clone.check_and_update();
                    thread::sleep(Duration::from_millis(10));
                }
            });
            handles.push(handle);
        }
        
        // Wait for all threads
        for handle in handles {
            handle.join().unwrap();
        }
        
        // Check that we didn't exceed limits
        let stats = limiter.get_stats();
        assert!(stats.requests_today <= 100);
    }
}