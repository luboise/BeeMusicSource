use super::*;

#[test]
fn mono_sample_index_test() {
    let bpm_changes = vec![BPMChange {
        time_point: TimePoint::new(0, 0.0),
        bpm: 160.0,
    }];

    let tp = TimePoint::new(0, 0.0);
    assert_eq!(0, tp.mono_sample_index(44100, &bpm_changes));

    let tp = TimePoint::new(1, 0.0);
    assert_eq!(66_150, tp.mono_sample_index(44100, &bpm_changes));
}

#[test]
fn ratio_test() {
    let start = TimePoint::new(0, 0.7);
    let end = start + TimePoint::new(4, 0.0);

    let res = start.ratio(&end, 0.5);
    assert_eq!(f64::from(res), 2.7);
}

#[test]
fn add_test() {
    let lhs = TimePoint::new(8, 0.0);
    let rhs = -TimePoint::new(2, 0.3);

    assert_eq!(f64::from(lhs + rhs), 5.7);
}

#[test]
fn sub_test() {
    let lhs = TimePoint::new(8, 0.0);
    let rhs = TimePoint::new(2, 0.3);

    assert_eq!(f64::from(lhs - rhs), 5.7);
}

#[test]
fn sub_test_2() {
    let right = TimePoint::new(5, 0.5);
    let left = TimePoint::new(2, 0.6);

    assert_eq!(
        right - left,
        TimePoint {
            measure: 2,
            submeasure: 0.9
        }
    );
}

#[test]
fn neg_test() {
    let tp = TimePoint {
        measure: 2,
        submeasure: 0.3,
    };

    assert_eq!(f64::from(-tp), -2.3);
}
