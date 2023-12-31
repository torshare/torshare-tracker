use async_trait::async_trait;
use bytes::Bytes;

use super::State;
use crate::{
    models::torrent::TorrentStatsList,
    storage::Processor,
    worker::{Result, TaskOutput},
};

pub type Input = Box<dyn FullScrapeProcessor>;
pub type Output = Box<dyn FullScrapeProcessor>;

pub struct TaskExecutor;

#[async_trait]
impl super::TaskExecutor for TaskExecutor {
    type Input = Input;
    type Output = Output;

    async fn execute(&self, mut input: Self::Input, state: State) -> Result<TaskOutput> {
        let processor: &mut dyn Processor<TorrentStatsList> = input.as_processor();
        let _ = state.storage.get_all_torrent_stats(processor).await?;

        Ok(TaskOutput::FullScrape(input))
    }
}

pub trait FullScrapeProcessor: Processor<TorrentStatsList> + Send + Sync {
    fn as_processor(&mut self) -> &mut dyn Processor<TorrentStatsList>;
    fn output(&mut self) -> Option<Bytes>;
}
