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
    fmt::{Debug, Display},
    future::Future,
    ops::{ControlFlow, Mul},
    time::Duration,
};

use derive_more::Deref;
use futures_retry_policies::{
    tokio::{retry, RetryFuture},
    RetryPolicy,
};
use rand::random;
use serde::{Deserialize, Serialize};
use tracing::warn;

use crate::serde::serde_duration_in_config;

#[derive(Clone, Debug, Deserialize, Serialize)]
#[cfg_attr(test, serde(deny_unknown_fields))]
pub struct ExponentialJitterRetryPolicyConfig {
    pub max_retries: u8,
    #[serde(with = "serde_duration_in_config")]
    pub step_size: Duration,
    #[serde(with = "serde_duration_in_config")]
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

    pub fn retry<MakeFut, Fut>(self, futures: MakeFut) -> RetryFuture<Self, MakeFut, Fut>
    where
        Self: RetryPolicy<Fut::Output>,
        MakeFut: FnMut() -> Fut,
        Fut: Future,
    {
        retry(self, futures)
    }

    fn register_pending_retry(&mut self, error: &dyn Display) -> Option<Duration> {
        if self.retry_count >= self.max_retries {
            return None;
        }

        warn!({error=%error}, "retrying request");

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
    E: Display,
{
    fn should_retry(&mut self, result: Result<T, E>) -> ControlFlow<Result<T, E>, Duration> {
        if let Err(error) = &result {
            if (self.retry_filter)(error) {
                if let Some(backoff) = self.register_pending_retry(error) {
                    return ControlFlow::Continue(backoff);
                }
            }
        }
        ControlFlow::Break(result)
    }
}
