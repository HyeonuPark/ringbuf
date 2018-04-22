use super::*;

use std::{isize, usize};

#[test]
fn test_counter_split() {
    let zero = Counter::new();

    assert_eq!(zero.split(), (false, 0));
    assert_eq!((zero + 1).split(), (false, 1));
    assert_eq!((zero + MSB).split(), (true, 0));
    assert_eq!((zero + !MSB).split(), (false, !MSB));
    assert_eq!((zero + usize::MAX).split(), (true, !MSB));
}

#[test]
fn test_compare_counters() {
    let zero = Counter::new();

    assert_eq!(zero, zero);
    assert!(zero <= zero);
    assert!(zero >= zero);
    assert!(zero < zero + 1);
    assert!(zero <= zero + 1);

    let mut one = zero;
    one += 1;
    assert_eq!(one, zero + 1);

    assert_eq!(one - zero, 1);
    assert_eq!(zero - zero, 0);
}

#[test]
fn test_compare_overflowed_counters() {
    let imax = isize::MAX as usize;

    let zero1 = Counter::new();
    let imax1 = zero1 + imax;
    let zero2 = imax1 + 1;
    let imax2 = zero2 + imax;

    assert_eq!(zero1.split(), (false, 0));
    assert_eq!(imax1.split(), (false, imax));
    assert_eq!(zero2.split(), (true, 0));
    assert_eq!(imax2.split(), (true, imax));

    assert_eq!(zero1, zero1);
    assert_eq!(imax1, imax1);
    assert_eq!(zero2, zero2);
    assert_eq!(imax2, imax2);

    assert!(zero1 > zero2);
    assert!(zero2 > zero1);
    assert!(imax1 > imax2);
    assert!(imax2 > imax1);

    assert!(zero1 < imax1);
    assert!(imax1 > zero1);
    assert!(zero2 < imax2);
    assert!(imax2 > zero2);

    assert!(zero1 > imax2);
    assert!(imax2 > zero1);
    assert!(zero2 > imax1);
    assert!(imax1 > zero2);
}

#[test]
fn test_multithread_counter_incr() {
    use std::sync::Arc;
    use std::thread;

    let counter = Arc::new(AtomicCounter::new());

    let handles: Vec<_> = (0..8).map(|_| {
        let counter = counter.clone();

        thread::spawn(move|| {
            for _ in 0..8000 {
                counter.incr(1);
            }
        })
    }).collect();

    for handle in handles {
        handle.join().unwrap();
    }

    assert_eq!(counter.fetch(), Counter::new() + 64000);
}
