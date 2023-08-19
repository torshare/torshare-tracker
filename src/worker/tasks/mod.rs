pub(super) mod announce;
pub(super) mod full_scrape;
pub(super) mod scrape;

use super::{Result, TaskOutput};
use crate::{config::TSConfig, storage::Storage};
use async_trait::async_trait;
use std::sync::Arc;

pub(super) fn err<T>(msg: &str) -> Result<T> {
    Err(msg.into())
}

#[async_trait]
pub(super) trait TaskExecutor: Send + Sync {
    type Input;
    type Output;

    async fn execute(&self, task: Self::Input, state: State) -> Result<TaskOutput>;
}

#[derive(Clone)]
pub struct State {
    pub storage: Arc<dyn Storage>,
    pub config: Arc<TSConfig>,
}
