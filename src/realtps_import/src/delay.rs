use crate::Job;
use crate::Chain;
use anyhow::Result;
use log::{debug, warn};
use rand::{
    self,
    distributions::{Distribution, Uniform},
};
use std::future::Future;
use std::pin::Pin;
use tokio::time::{self, Duration};

async fn delay(base_ms: u64) {
    let jitter = Uniform::from(0..10);
    let delay_msecs = base_ms + jitter.sample(&mut rand::thread_rng());
    let delay_time = Duration::from_millis(delay_msecs);
    time::sleep(delay_time).await;
}

/// The pace we want to request blocks at.
///
/// FIXME rename
pub fn block_pace(chain: Chain) -> u64 {
    let msecs = match chain {
        Chain::Elrond => 1000,   // 6s block time
        Chain::Optimism => 1000, // got blocked at 500ms, unclear what rate they want
        // Need to go fast to keep up.
        // Solana's RpcClient will use its built in rate limiter when connecting to public nodes.
        Chain::Solana => 0,
        _ => 500,
    };

    msecs
}

/// How long to wait between imports.
///
/// This should be somewhat longer than the average block production time to
/// avoid making requests for new blocks when there are non.
pub async fn rescan_delay(chain: Chain) {
    let delay_secs = match chain {
        Chain::InternetComputer => 1,
        Chain::Solana => 1, // Need to go fast to keep up
        Chain::Polkadot => 7, // 6s block time, server rate-limited, can't wait too long
        Chain::Kusama => 7, // "
        Chain::Optimism => 10, // Unclear, just experimenting
        _ => 30,
    };
    let msecs = 1000 * delay_secs;
    debug!("delaying {} ms to rescan chain {}", msecs, chain);
    delay(msecs).await
}

pub async fn job_error_delay(job: &Job) {
    let msecs = 1000;
    debug!("delaying {} ms to retry job {:?}", msecs, job);
    delay(msecs).await;
}

pub async fn recalculate_delay() {
    let msecs = 5000;
    debug!("delaying {} ms before recaclulating", msecs);
    delay(msecs).await;
}

pub async fn remove_data_delay() {
    let msecs = 60 * 60 * 24 * 1000;
    debug!("delaying {} ms to remove old blocks", msecs);
    delay(msecs).await;
}

pub async fn retry_if_err<'caller, F, T>(chain: Chain, f: F) -> Result<T>
where
    F: Fn() -> Pin<Box<dyn Future<Output = Result<T>> + Send + 'caller>>,
{
    let tries = 3;
    let base_delay_ms = 500;
    let mut try_num = 1;
    let r = loop {
        let r = f().await;
        match r {
            Ok(r) => break Ok(r),
            Err(e) => {
                if try_num == tries {
                    break Err(e);
                } else {
                    let delay_ms = base_delay_ms * try_num;
                    warn!(
                        "for chain {} received err {}. retrying in {} ms",
                        chain, e, delay_ms
                    );
                    delay(delay_ms).await;
                }
            }
        }
        try_num += 1;
    };
    r
}

pub async fn retry_if_none<'caller, F, T>(chain: Chain, f: F) -> Result<Option<T>>
where
    F: Fn() -> Pin<Box<dyn Future<Output = Result<Option<T>>> + Send + 'caller>>,
{
    let tries = 3;
    let base_delay_ms = 500;
    let mut try_num = 1;
    let r = loop {
        let r = f().await?;
        match r {
            Some(r) => break Ok(Some(r)),
            None => {
                if try_num == tries {
                    break Ok(None);
                } else {
                    let delay_ms = base_delay_ms * try_num;
                    warn!("for chain {} received None. retrying in {} ms", chain, delay_ms);
                    delay(delay_ms).await;
                }
            }
        }
        try_num += 1;
    };
    r
}
