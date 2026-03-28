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
