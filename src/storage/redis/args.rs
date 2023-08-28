use crate::models::{
    common::{InfoHash, IpType, INFOHASH_LENGTH},
    peer::{Peer, PeerType},
    torrent::Torrent,
};
use bytes::{Bytes, BytesMut};
use redis::{FromRedisValue, RedisError, RedisResult, ToRedisArgs, Value};
use std::{array::TryFromSliceError, mem};

pub const REDIS_KEY_PREFIX: &[u8] = b"ts_";
pub const TORRENT_KEY_PREFIX: &[u8] = REDIS_KEY_PREFIX;
pub const TORRENT_KEY_LEN: usize = TORRENT_KEY_PREFIX.len() + INFOHASH_LENGTH * 2;
pub const TYPE_IPV4: &[u8] = b"_v4";
pub const TYPE_IPV6: &[u8] = b"_v6";
pub const TYPE_LEN: usize = 3;
pub const SWARM_KEY_SEEDER_PREFIX: &[u8] = b"_s";
pub const SWARM_KEY_LEECHER_PREFIX: &[u8] = b"_l";
pub const SWARM_KEY_PARTIAL_SEED_PREFIX: &[u8] = b"_p";
pub const SWARM_KEY_LEN: usize = TORRENT_KEY_LEN + TYPE_LEN + 2;
pub const TORRENT_COMPLETED_KEY: &[u8] = b"c";

// Define the macro for serializing fields
macro_rules! serialize_field {
    ($out:ident, $field_name:expr, $field_value:expr) => {
        $field_name.write_redis_args($out);
        $field_value.write_redis_args($out);
    };
}

macro_rules! process_chunks_for_struct {
    ($chunks:expr, $struct:expr, $($pattern:expr => $field:ident),*) => {
        for item in $chunks {
            let field = &item[0];
            let value = &item[1];

            match field {
                Value::Data(ref field) => {
                    $(
                        if field.as_slice() == $pattern {
                            $struct.$field = FromRedisValue::from_redis_value(&value)?;
                        }
                    )*
                },
                _ => {}
            }
        }
    };
}

pub struct SwarmKey<'a> {
    pub torrent_key: &'a [u8],
    pub peer_type: PeerType,
    pub peer_ip_type: IpType,
}

impl<'a> SwarmKey<'a> {
    pub fn encode(&self) -> Bytes {
        let mut bytes = BytesMut::with_capacity(SWARM_KEY_LEN);
        bytes.extend_from_slice(self.torrent_key);

        match self.peer_ip_type {
            IpType::V4 => bytes.extend_from_slice(TYPE_IPV4),
            IpType::V6 => bytes.extend_from_slice(TYPE_IPV6),
        }

        match self.peer_type {
            PeerType::Seeder => bytes.extend_from_slice(SWARM_KEY_SEEDER_PREFIX),
            PeerType::Leecher => bytes.extend_from_slice(SWARM_KEY_LEECHER_PREFIX),
            PeerType::Partial => bytes.extend_from_slice(SWARM_KEY_PARTIAL_SEED_PREFIX),
        }

        bytes.freeze()
    }

    pub fn get_all_swarm_keys(torrent_key: &'a [u8], ip_type: IpType) -> (Self, Self, Self) {
        let swark_key_leecher = SwarmKey {
            torrent_key,
            peer_type: PeerType::Leecher,
            peer_ip_type: ip_type,
        };

        let swark_key_seeder = SwarmKey {
            torrent_key,
            peer_type: PeerType::Seeder,
            peer_ip_type: ip_type,
        };

        let swark_key_partial = SwarmKey {
            torrent_key,
            peer_type: PeerType::Partial,
            peer_ip_type: ip_type,
        };

        (swark_key_leecher, swark_key_seeder, swark_key_partial)
    }
}

pub struct TorrentKey<'a>(pub &'a InfoHash);

impl<'a> TorrentKey<'a> {
    pub fn encode(&self) -> Bytes {
        let mut bytes = BytesMut::with_capacity(TORRENT_KEY_LEN);
        bytes.extend_from_slice(TORRENT_KEY_PREFIX);
        bytes.extend_from_slice(self.0.to_string().as_bytes());
        bytes.freeze()
    }
}

impl<'a> ToRedisArgs for TorrentKey<'a> {
    fn write_redis_args<W>(&self, out: &mut W)
    where
        W: ?Sized + redis::RedisWrite,
    {
        out.write_arg(&self.encode());
    }
}

impl<'a> ToRedisArgs for SwarmKey<'a> {
    fn write_redis_args<W>(&self, out: &mut W)
    where
        W: ?Sized + redis::RedisWrite,
    {
        out.write_arg(&self.encode());
    }
}

const EXPIRE_AT_SIZE: usize = mem::size_of::<u64>();

impl ToRedisArgs for Peer {
    fn write_redis_args<W>(&self, out: &mut W)
    where
        W: ?Sized + redis::RedisWrite,
    {
        let addr = self.addr.as_bytes();
        let expire_at = self.expire_at.as_secs();

        let len = EXPIRE_AT_SIZE + addr.len();
        let mut bytes = BytesMut::with_capacity(len);

        bytes.extend_from_slice(&expire_at.to_be_bytes());
        bytes.extend_from_slice(self.addr.as_bytes());

        out.write_arg(&bytes);
    }

    fn is_single_arg(&self) -> bool {
        true
    }
}

impl FromRedisValue for Peer {
    fn from_redis_value(v: &Value) -> RedisResult<Self> {
        match *v {
            Value::Data(ref bytes) => {
                let expire_at = ts_utils::time::Duration::from_secs(u64::from_be_bytes(
                    bytes[..EXPIRE_AT_SIZE].try_into().map_err(from_slice_err)?,
                ));

                let addr = bytes[EXPIRE_AT_SIZE..].try_into().map_err(from_str)?;
                let peer = Peer { addr, expire_at };

                Ok(peer)
            }
            _ => Err((redis::ErrorKind::TypeError, "Unexpected type").into()),
        }
    }
}

impl ToRedisArgs for Torrent {
    fn write_redis_args<W>(&self, out: &mut W)
    where
        W: ?Sized + redis::RedisWrite,
    {
        serialize_field!(out, TORRENT_COMPLETED_KEY, self.completed);
    }

    fn is_single_arg(&self) -> bool {
        false
    }
}

impl FromRedisValue for Torrent {
    fn from_redis_value(v: &redis::Value) -> RedisResult<Self> {
        match *v {
            Value::Bulk(ref items) => {
                let mut torrent = Torrent::default();
                let chunks = items.chunks(2);

                process_chunks_for_struct!(
                    chunks,
                    torrent,
                    TORRENT_COMPLETED_KEY => completed
                );

                Ok(torrent)
            }
            _ => Err((redis::ErrorKind::TypeError, "Unexpected type").into()),
        }
    }
}

fn from_slice_err(_: TryFromSliceError) -> RedisError {
    (redis::ErrorKind::TypeError, "Unexpected type").into()
}

fn from_str(msg: &'static str) -> RedisError {
    (redis::ErrorKind::TypeError, msg).into()
}
