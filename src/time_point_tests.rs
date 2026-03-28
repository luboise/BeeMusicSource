use super::*;

#[test]
fn sub_test() {
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
