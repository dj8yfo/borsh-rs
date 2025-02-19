use alloc::{
    string::{String, ToString},
    vec,
    vec::Vec,
};

macro_rules! test_vec {
    ($v: expr, $t: ty, $snap: expr) => {
        let mut buf = vec![]; 

        borsh::to_writer_async(&mut buf, &$v).await.expect("Serialization failed");
        // this links check to snapshots in `../roundtrip/snapshots/vec_*.snap`
        #[cfg(feature = "std")]
        if $snap {
            let mut settings = insta::Settings::clone_current();
            settings.set_prepend_module_to_snapshot(false);
            settings.set_snapshot_path("../roundtrip/snapshots");
            settings.bind(|| {
                insta::assert_debug_snapshot!(buf);
            });
        }
        let mut reader = buf.as_slice();
        let actual_v: Vec<$t> = borsh::from_reader_async(&mut reader).await.expect("Deserialization failed");
        
        assert_eq!(actual_v, $v);
    };
}

macro_rules! test_vecs {
    ($test_name: ident, $el: expr, $t: ty) => {
        #[tokio::test]
        async fn $test_name() {
            test_vec!(Vec::<$t>::new(), $t, true);
            test_vec!(vec![$el], $t, true);
            test_vec!(vec![$el; 10], $t, true);
            test_vec!(vec![$el; 100], $t, true);
            test_vec!(vec![$el; 1000], $t, false); // one assumes that the concept has been proved
            test_vec!(vec![$el; 10000], $t, false);
        }
    };
}

test_vecs!(test_vec_u8, 100u8, u8);
test_vecs!(test_vec_i8, 100i8, i8);
test_vecs!(test_vec_u32, 1000000000u32, u32);
test_vecs!(test_vec_f32, 1000000000.0f32, f32);
test_vecs!(test_vec_string, "a".to_string(), String);
test_vecs!(test_vec_vec_u8, vec![100u8; 10], Vec<u8>);
test_vecs!(test_vec_vec_u32, vec![100u32; 10], Vec<u32>);
