use borsh::{from_slice, BorshSerialize};

macro_rules! test_primitive {
    ($test_name: ident, $v: expr, $t: ty) => {
        #[test]
        fn $test_name() {
            let expected: $t = $v;
            let buf = expected.try_to_vec().unwrap();
            let actual = from_slice::<$t>(&buf).expect("failed to deserialize");
            assert_eq!(actual, expected);
        }
    };
}

test_primitive!(test_isize_neg, -100isize, isize);
test_primitive!(test_isize_pos, 100isize, isize);
test_primitive!(test_isize_min, isize::min_value(), isize);
test_primitive!(test_isize_max, isize::max_value(), isize);

test_primitive!(test_usize, 100usize, usize);
test_primitive!(test_usize_min, usize::min_value(), usize);
test_primitive!(test_usize_max, usize::max_value(), usize);
