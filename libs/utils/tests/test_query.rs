use ts_utils::query::from_bytes;

#[derive(Debug, Default, PartialEq, serde::Deserialize)]
struct AnnounceRequest {
    info_hash: [u8; 20],
    peer_id: [u8; 20],
    port: u16,
    uploaded: u64,
    downloaded: u64,
    left: u64,
    event: String,
}

#[test]
fn test_from_bytes() {
    let input = b"info_hash=%d1%83%91%e2%88%83%e2%98%82%5e%e2%98%82%8a%e3%8f%90%e1%98%b8%ed%f2b&peer_id=1234567890ABCDEFGHIJ&port=6881&uploaded=123456789&downloaded=987654321&left=1234567890&event=started";

    let info_hash = hex::decode("d18391e28883e298825ee298828ae38f90e198b8")
        .unwrap()
        .try_into()
        .unwrap();

    let peer_id = "1234567890ABCDEFGHIJ".as_bytes().try_into().unwrap();

    let expected_req = AnnounceRequest {
        info_hash,
        peer_id,
        port: 6881,
        uploaded: 123456789,
        downloaded: 987654321,
        left: 1234567890,
        event: "started".to_string(),
    };

    let result: Result<AnnounceRequest, _> = from_bytes(input);
    assert!(result.is_ok());

    let req = result.unwrap();
    assert_eq!(req, expected_req);
}
