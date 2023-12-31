macro_rules! constant_strings {
    (
        $(
            $(#[$docs:meta])*
            ($name_upcase:ident, $value:expr),
        )+
    ) => {
        $(
            $(#[$docs])*
            #[allow(dead_code)]
            pub const $name_upcase: &'static str = $value;
        )+
    }
}

constant_strings! {
    // HTTP RESPONSE
    (NOT_FOUND, "Not Found"),
    (REQUEST_TIMEOUT, "Request Timeout"),
    (UNAUTHORIZED, "Unauthorized"),
    (BAD_REQUEST, "Bad request"),
    (INTERNAL_SERVER_ERROR, "An Error Occurred, Please Try Again!"),

    // TRACKER
    (TRACKER_RESPONSE_TRACKER_ID, "tracker id"),
    (TRACKER_RESPONSE_FAILURE_REASON, "failure reason"),
    (TRACKER_RESPONSE_FAILURE_CODE, "failure code"),
    (TRACKER_RESPONSE_WARNING_MESSAGE, "warning message"),
    (TRACKER_RESPONSE_INTERVAL, "interval"),
    (TRACKER_RESPONSE_MIN_INTERVAL, "min interval"),
    (TRACKER_RESPONSE_COMPLETE, "complete"),
    (TRACKER_RESPONSE_FILES, "files"),
    (TRACKER_RESPONSE_INCOMPLETE, "incomplete"),
    (TRACKER_RESPONSE_PEERS, "peers"),
    /// http://bittorrent.org/beps/bep_0007.html
    (TRACKER_RESPONSE_PEERS6, "peers6"),
    (TRACKER_RESPONSE_DOWNLOADED, "downloaded"),
    (TRACKER_RESPONSE_DOWNLOADERS, "downloaders"),
    (TRACKER_RESPONSE_CRYPTO_FLAGS, "crypto_flags"),
    (TRACKER_RESPONSE_RETRY_IN, "retry in"),
    (TRACKER_RESPONSE_PEER_ID, "peer id"),
    (TRACKER_RESPONSE_IP, "ip"),
    (TRACKER_RESPONSE_PORT, "port"),
    (TRACKER_RESPONSE_NEVER, "never"),

    // TRACKER ERRORS
    (TRACKER_ERROR_MISSING_INFOHASH, "missing info_hash"),
    (TRACKER_ERROR_MISSING_PEERID, "missing peer id"),
    (TRACKER_ERROR_MISSING_PORT, "missing port"),
    (TRACKER_ERROR_INVALID_INFOHASH, "invalid infohash: infohash is not 20 bytes long"),
    (TRACKER_ERROR_INVALID_PEERID, "invalid peerid: peerid is not 20 bytes long"),
    (TRACKER_ERROR_NOT_FOUND_TORRENT, "torrent not found"),
    (TRACKER_ERROR_TOO_MANY_REQUEST, "a request was sent before the specified time"),
    (TRACKER_ERROR_NOT_TRACKER, "not a tracker"),
    (TRACKER_ERROR_PEER_LIST_NOT_SUPPORTED, "peer list response is not supported"),
    (TRACKER_ERROR_INVALID_ANNOUNCE_REQUEST, "invalid announce request"),
    (TRACKER_ERROR_FULL_SCRAPE_NOT_ALLOWED, "full scrape not allowed"),
    (TRACKER_ERROR_UNREGISTERED_TORRENT_PASS, "unregistered torrent pass"),
    (TRACKER_ERROR_UNREGISTERED_TORRENT, "unregistered torrent"),
    (TRACKER_ERROR_BLOCKED_INFOHASH, "blocked infohash"),
    (TRACKER_ERROR_BLOCKED_CLIENT, "blocked client"),
    (TRACKER_ERROR_BLOCKED_IP, "blocked ip"),
    (TRACKER_ERROR_HTTP_SCRAPE_NOT_ALLOWED, "http scrape not allowed"),
    (TRACKER_ERROR_HTTP_ANNOUNCE_NOT_ALLOWED, "http announce not allowed"),
}
