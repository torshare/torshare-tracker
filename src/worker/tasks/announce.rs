use super::{err, State};
use crate::{
    config::TSConfig,
    constants,
    models::{
        common::{IpType, NumOfBytes, PEER_ID_LENGTH},
        peer::{Peer, PeerType, PEER_ADDR_V4_LENGTH, PEER_ADDR_V6_LENGTH},
        torrent::{PeerDict, PeerIdKey},
        tracker::{
            AnnounceEvent, AnnounceRequest, AnnounceResponse, NonCompactPeer, ResponsePeerList,
        },
    },
    storage::Processor,
    worker::{Result, TaskOutput},
};
use async_trait::async_trait;
use bytes::BytesMut;
use std::{net::IpAddr, ops::Range};

pub struct TaskExecutor;

pub type Input = (AnnounceRequest, IpAddr);
pub type Output = AnnounceResponse;

const NUM_ZERO: NumOfBytes = NumOfBytes(0);

#[async_trait]
impl super::TaskExecutor for TaskExecutor {
    type Input = Input;
    type Output = Output;

    async fn execute(&self, input: Self::Input, state: State) -> Result<TaskOutput> {
        let (req, sender_addr) = input;
        let storage = state.storage;
        let config = state.config;

        if config.infohash_blocklist.contains(&req.info_hash) {
            return err(constants::TRACKER_ERROR_BLOCKED_INFOHASH);
        }

        let info_hash = &req.info_hash;
        if !storage.has_torrent(info_hash).await? {
            match config.auto_register_torrent() {
                true => storage.insert_torrent(info_hash, None).await?,
                false => return err(constants::TRACKER_ERROR_NOT_FOUND_TORRENT),
            };
        }

        let peer: Peer = (&req, sender_addr).try_into()?;
        let announce_time = peer.last_announce_at as usize;

        let mut peer_type = {
            if peer.left == NUM_ZERO {
                PeerType::Seeder
            } else {
                PeerType::Leecher
            }
        };

        let user_key = req.key.as_ref().map(|k| k.as_ref());
        let peer_id_key = PeerIdKey::new(&req.peer_id, user_key);

        match req.event {
            Some(AnnounceEvent::Started) => {
                storage
                    .put_peer_in_swarm(info_hash, &peer_id_key, peer, peer_type)
                    .await?;
            }

            Some(AnnounceEvent::Stopped) => {
                storage
                    .remove_peer_from_swarm(info_hash, &peer_id_key, peer_type)
                    .await?;
            }

            Some(AnnounceEvent::Completed) => {
                if !matches!(peer_type, PeerType::Seeder) {
                    return err(constants::TRACKER_ERROR_INVALID_ANNOUNCE_REQUEST);
                }

                storage
                    .promote_peer_in_swarm(info_hash, &peer_id_key, peer)
                    .await?;
            }

            Some(AnnounceEvent::Paused) => {
                peer_type = PeerType::Partial;
                storage
                    .update_or_put_peer_in_swarm(info_hash, &peer_id_key, peer, peer_type)
                    .await?;
            }

            _ => {
                storage
                    .update_or_put_peer_in_swarm(info_hash, &peer_id_key, peer, peer_type)
                    .await?;
            }
        }

        let response = {
            let mut peers = None;
            let mut peers6 = None;
            let mut seeders = 0;
            let mut leechers = 0;
            let warning_message = None;

            if req.event != Some(AnnounceEvent::Stopped) {
                let sender_ip_type = if sender_addr.is_ipv4() {
                    IpType::V4
                } else {
                    IpType::V6
                };

                let mut processor = ResponsePeersExtractor::new(
                    &req,
                    &peer_id_key,
                    sender_ip_type,
                    announce_time,
                    &config,
                );

                let (stats, _) = tokio::try_join! {
                    storage.get_torrent_stats(info_hash),
                    storage.extract_peers_from_swarm(info_hash, peer_type, &mut processor)
                }?;

                (peers, peers6) = processor.into_output();

                if let Some(stats) = stats {
                    seeders = stats.seeders;
                    leechers = stats.leechers;
                }
            }

            let interval = config.announce_interval();
            let min_interval = config.min_announce_interval();

            AnnounceResponse {
                peers,
                peers6,
                leechers,
                seeders,
                warning_message,
                interval,
                min_interval,
            }
        };

        Ok(TaskOutput::Announce(response))
    }
}

struct ResponsePeersExtractor<'a> {
    req: &'a AnnounceRequest,
    peer_id_key: &'a PeerIdKey,
    sender_ip_type: IpType,
    numwant: usize,
    peers: PeersOutput,
    peer_count: usize,
    announce_time: usize,
}

impl<'a> ResponsePeersExtractor<'a> {
    fn new(
        req: &'a AnnounceRequest,
        peer_id_key: &'a PeerIdKey,
        sender_ip_type: IpType,
        announce_time: usize,
        config: &TSConfig,
    ) -> Self {
        let numwant = std::cmp::min(
            req.numwant.unwrap_or(config.default_numwant()),
            config.max_numwant(),
        ) as usize;

        let peers = PeersOutput::new(req.compact, numwant, sender_ip_type);

        Self {
            numwant,
            req,
            peer_id_key,
            sender_ip_type,
            peers,
            announce_time,
            peer_count: 0,
        }
    }

    fn extract(&mut self, peer_dict: &PeerDict, range: Range<usize>) -> bool {
        for (peer_id_key, peer) in &peer_dict[range] {
            if self.peer_count >= self.numwant {
                return false;
            }

            if !self.predicate(peer, peer_id_key) {
                continue;
            }

            if self
                .peers
                .insert(peer_id_key, peer, self.sender_ip_type, self.req.no_peer_id)
            {
                self.peer_count += 1;
            }
        }

        return true;
    }

    fn predicate(&self, peer: &Peer, peer_id_key: &PeerIdKey) -> bool {
        if !match self.sender_ip_type {
            IpType::V4 => peer.is_ipv4(),
            IpType::V6 => peer.is_ipv6(),
        } {
            return false;
        }

        if peer_id_key == self.peer_id_key {
            return false;
        }

        return true;
    }

    fn into_output(self) -> (Option<ResponsePeerList>, Option<ResponsePeerList>) {
        let peers = match self.peers {
            PeersOutput::Compact(bytes) => {
                if bytes.is_empty() {
                    None
                } else {
                    Some(ResponsePeerList::Compact(bytes.to_vec()))
                }
            }
            PeersOutput::NonCompact(peers) => {
                if peers.is_empty() {
                    None
                } else {
                    Some(ResponsePeerList::NonCompact(peers))
                }
            }
        };

        match self.sender_ip_type {
            IpType::V4 => (peers, None),
            IpType::V6 => (None, peers),
        }
    }
}

impl<'a> Processor<PeerDict> for ResponsePeersExtractor<'a> {
    fn process(&mut self, peer_dict: &PeerDict) -> bool {
        let total_peers = peer_dict.len();
        if (self.numwant - self.peer_count) >= total_peers {
            return self.extract(peer_dict, 0..total_peers);
        }

        let start = self.announce_time % total_peers;
        match self.extract(peer_dict, start..total_peers) {
            true => self.extract(peer_dict, 0..start),
            false => false,
        }
    }
}

enum PeersOutput {
    Compact(BytesMut),
    NonCompact(Vec<NonCompactPeer>),
}

impl PeersOutput {
    fn new(is_compact: bool, numwant: usize, ip_type: IpType) -> Self {
        match is_compact {
            true => {
                let capacity = match ip_type {
                    IpType::V4 => numwant * PEER_ADDR_V4_LENGTH,
                    IpType::V6 => numwant * PEER_ADDR_V6_LENGTH,
                };

                PeersOutput::Compact(BytesMut::with_capacity(capacity))
            }
            false => PeersOutput::NonCompact(Vec::with_capacity(numwant)),
        }
    }

    fn insert(
        &mut self,
        peer_id_key: &PeerIdKey,
        peer: &Peer,
        sender_ip_type: IpType,
        no_peer_id: bool,
    ) -> bool {
        match self {
            PeersOutput::Compact(peer_list) => {
                if let Some(addr) = peer.addr_as_bytes(sender_ip_type) {
                    peer_list.extend_from_slice(&addr);
                    return true;
                }
            }

            PeersOutput::NonCompact(peer_list) => {
                if let Some((ip, port)) = match sender_ip_type {
                    IpType::V4 => peer.addr_v4.as_ref().map(|addr| addr.into()),
                    IpType::V6 => peer.addr_v6.as_ref().map(|addr| addr.into()),
                } {
                    let peer_id = match no_peer_id {
                        true => None,
                        false => peer_id_key.as_ref()[..PEER_ID_LENGTH].try_into().ok(),
                    };

                    peer_list.push(NonCompactPeer { ip, port, peer_id });

                    return true;
                }
            }
        }

        return false;
    }
}