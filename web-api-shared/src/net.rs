// Copyright 2023 Xayn AG
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as
// published by the Free Software Foundation, version 3.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.

//! Networking related utilities.

use std::{
    ops::{ControlFlow, Mul},
    time::Duration,
};

use derive_more::Deref;
use futures_retry_policies::RetryPolicy;
use rand::random;

#[derive(Debug, Clone)]
pub struct ExponentialJitterRetryPolicyConfig {
    pub max_retries: u8,
    pub step_size: Duration,
    pub max_backoff: Duration,
}

#[derive(Deref)]
pub struct ExponentialJitterRetryPolicy<F> {
    #[deref]
    config: ExponentialJitterRetryPolicyConfig,
    retry_filter: F,
    retry_count: u8,
}

impl ExponentialJitterRetryPolicy<fn(&'_ anyhow::Error) -> bool> {
    pub fn new(config: ExponentialJitterRetryPolicyConfig) -> Self {
        Self {
            config,
            retry_count: 0,
            retry_filter: |_err| true,
        }
    }
}

impl<F> ExponentialJitterRetryPolicy<F> {
    pub fn with_retry_filter<F2, R>(self, retry_filter: F2) -> ExponentialJitterRetryPolicy<F2>
    where
        F2: Fn(&R) -> bool,
    {
        ExponentialJitterRetryPolicy {
            config: self.config,
            retry_count: self.retry_count,
            retry_filter,
        }
    }

    fn register_pending_retry(&mut self) -> Option<Duration> {
        if self.retry_count >= self.max_retries {
            return None;
        }

        let duration = self
            .step_size
            // exponential backoff
            .mul(2u32.saturating_pow(self.retry_count as u32))
            // upper limit for backoff sleep time
            .min(self.max_backoff)
            // jitter
            .mul_f32(random());
        self.retry_count += 1;
        Some(duration)
    }
}

impl<F, T, E> RetryPolicy<Result<T, E>> for ExponentialJitterRetryPolicy<F>
where
    F: FnMut(&E) -> bool,
{
    fn should_retry(&mut self, result: Result<T, E>) -> ControlFlow<Result<T, E>, Duration> {
        if matches!(&result, Err(err) if (self.retry_filter)(err)) {
            if let Some(backoff) = self.register_pending_retry() {
                return ControlFlow::Continue(backoff);
            }
        }
        ControlFlow::Break(result)
    }
}
