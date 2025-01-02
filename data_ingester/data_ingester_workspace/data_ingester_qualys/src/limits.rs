use reqwest::header::HeaderMap;
use tokio::time::{sleep, Duration};
use tracing::debug;

/// Limits to use when throttling Qualys requests
#[derive(Debug, PartialEq, Eq)]
#[allow(dead_code)]
pub(crate) struct QualysLimits {
    rate_limit: usize,
    rate_window_seconds: usize,
    rate_remaining: usize,
    rate_to_wait_seconds: usize,
    concurrency_limit: usize,
    concurrency_running: usize,
}

impl Default for QualysLimits {
    /// Express/Consultant
    /// API Service Concurrency Limit per Subscription (per API): 1 call
    /// Rate Limit per Subscription (per API): 50 calls per Day
    ///
    /// Standard API
    /// Service Concurrency Limit per Subscription (per API): 2 calls
    /// Rate Limit per Subscription (per API): 300 calls per Hour
    ///
    /// Enterprise API Service
    /// Concurrency Limit per Subscription (per API): 5 calls
    /// Rate Limit per Subscription (per API): 750 calls per Hour
    ///
    /// Premium API Service
    /// Concurrency Limit per Subscription (per API): 10 calls
    /// Rate Limit per Subscription (per API): 2000 calls per Hour
    ///
    /// https://cdn2.qualys.com/docs/qualys-api-limits.pdf
    ///
    fn default() -> Self {
        Self {
            rate_limit: 300,
            rate_window_seconds: 60 * 60,
            rate_remaining: 300,
            rate_to_wait_seconds: 0,
            concurrency_limit: 2,
            concurrency_running: 0,
        }
    }
}

impl QualysLimits {
    /// Extract a limit header or provide a default value
    /// TODO check default value is sane
    fn get_usize_from_header(headers: &HeaderMap, key: &str) -> usize {
        static DEFAULT: usize = 0;
        headers
            .get(key)
            .map(|h| {
                h.to_str()
                    .unwrap_or_default()
                    .parse::<usize>()
                    .unwrap_or(DEFAULT)
            })
            .unwrap_or(DEFAULT)
    }

    /// Extract limit headers from a [reqwest::HeaderMap]
    pub(crate) fn from_headers(headers: &HeaderMap) -> Self {
        debug!("Qualys response headers: {:?}", headers);
        let limits = Self {
            rate_limit: QualysLimits::get_usize_from_header(headers, "X-RateLimit-Limit"),
            rate_window_seconds: QualysLimits::get_usize_from_header(
                headers,
                "X-RateLimit-Window-Sec",
            ),
            rate_remaining: QualysLimits::get_usize_from_header(headers, "X-RateLimit-Remaining"),
            rate_to_wait_seconds: QualysLimits::get_usize_from_header(
                headers,
                "X-RateLimit-ToWait-Sec",
            ),
            concurrency_limit: QualysLimits::get_usize_from_header(
                headers,
                "X-Concurrency-Limit-Limit",
            ),
            concurrency_running: QualysLimits::get_usize_from_header(
                headers,
                "X-Concurrency-Limit-Running",
            ),
        };
        debug!("Qualys parsed limits: {:?}", limits);
        limits
    }

    /// Wait for the rate limit to expire
    pub(crate) async fn wait(&self) {
        if self.rate_remaining < 1 || self.rate_to_wait_seconds > 0 {
            sleep(Duration::from_secs(self.rate_to_wait_seconds as u64)).await;
        }
    }
}

#[cfg(test)]
mod test {
    use crate::limits::QualysLimits;
    use reqwest::header::HeaderMap;
    use std::time::Instant;

    #[tokio::test]
    async fn test_qualys_limits_get_usize_from_header() {
        let mut header_map = HeaderMap::new();

        let _ = header_map.insert("test", "1".parse().unwrap());
        let result = QualysLimits::get_usize_from_header(&header_map, "test");
        assert_eq!(1, result);
    }

    #[tokio::test]
    async fn test_qualys_limits_from_headers() {
        let mut header_map = HeaderMap::new();

        let _ = header_map.insert("X-RateLimit-Limit", "1".parse().unwrap());
        let _ = header_map.insert("X-RateLimit-Window-Sec", "2".parse().unwrap());
        let _ = header_map.insert("X-RateLimit-Remaining", "3".parse().unwrap());
        let _ = header_map.insert("X-RateLimit-ToWait-Sec", "4".parse().unwrap());
        let _ = header_map.insert("X-Concurrency-Limit-Limit", "5".parse().unwrap());
        let _ = header_map.insert("X-Concurrency-Limit-Running", "6".parse().unwrap());

        let result = QualysLimits::from_headers(&header_map);
        let expected = QualysLimits {
            rate_limit: 1,
            rate_window_seconds: 2,
            rate_remaining: 3,
            rate_to_wait_seconds: 4,
            concurrency_limit: 5,
            concurrency_running: 6,
        };
        assert_eq!(expected, result);
    }

    #[tokio::test]
    async fn test_qualys_limits_should_wait() {
        let limits = QualysLimits {
            rate_to_wait_seconds: 1,
            rate_remaining: 0,
            ..Default::default()
        };

        let before = Instant::now();
        limits.wait().await;
        let after = Instant::now();

        let duration = after - before;
        assert!(duration > tokio::time::Duration::from_secs(1));
    }

    #[tokio::test]
    async fn test_qualys_limits_should_not_wait() {
        let limits = QualysLimits {
            rate_to_wait_seconds: 1,
            rate_remaining: 0,
            ..Default::default()
        };

        let before = Instant::now();
        limits.wait().await;
        let after = Instant::now();

        let duration = after - before;
        assert!(duration < tokio::time::Duration::from_secs(1));
    }
}
