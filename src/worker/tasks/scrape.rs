use async_trait::async_trait;

use super::State;
use crate::{
    models::tracker::{ScrapeRequest, ScrapeResponse},
    worker::{Result, TaskOutput},
};

pub type Input = ScrapeRequest;
pub type Output = ScrapeResponse;

pub struct TaskExecutor;

#[async_trait]
impl super::TaskExecutor for TaskExecutor {
    type Input = Input;
    type Output = Output;

    async fn execute(&self, input: Self::Input, state: State) -> Result<TaskOutput> {
        let files = state
            .storage
            .get_multi_torrent_stats(input.info_hashes)
            .await?;

        let output = ScrapeResponse::new(files);

        Ok(TaskOutput::Scrape(output))
    }
}
