use webrtc_socket::{blocking::BlockingWebRTCSocket, peer::RtcConfigBuilder};

use crate::helper::enable_tracing;

#[test]
fn blocking() {
    enable_tracing();
    let rtc_config = RtcConfigBuilder::new()
        .address("127.0.0.1")
        .port(3657)
        .user("bob")
        .password("bob")
        .build();
    let mut s = BlockingWebRTCSocket::connect(rtc_config).unwrap();
    let _ = s.ggrs_socket();

    assert!(true);
}
