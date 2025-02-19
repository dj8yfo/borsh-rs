use std::net::{IpAddr, Ipv4Addr};

#[tokio::test]
async fn test_ipv4_addr_roundtrip_enum() {
    let original = IpAddr::V4(Ipv4Addr::new(192, 168, 0, 1));

    let mut encoded = vec![]; 


    borsh::to_writer_async(&mut encoded, &original).await.expect("Serialization failed");

    // this links check to snapshot in `../roundtrip/snapshots/ipv4_addr_roundtrip_enum.snap`
    #[cfg(feature = "std")]
    {
        let mut settings = insta::Settings::clone_current();
        settings.set_prepend_module_to_snapshot(false);
        settings.set_snapshot_path("../roundtrip/snapshots");
        settings.bind(|| {
            insta::assert_debug_snapshot!(encoded);
        });
    }


    let mut reader = encoded.as_slice();
    let decoded: IpAddr = borsh::from_reader_async(&mut reader).await.expect("Deserialization failed");
    assert_eq!(original, decoded);
}

