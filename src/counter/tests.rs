use super::*;

#[test]
fn test_compare_counters() {
    let zero = Counter::new(0);

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
    assert_eq!(zero - one, -1);
}

#[test]
fn test_compare_overflowed_counters() {
    const STEP: usize = COUNTER_VALID_RANGE;

    let step0 = Counter::new(0);
    let step1 = step0 + STEP;
    let step2 = step1 + STEP;
    let step3 = step2 + STEP;

    assert_eq!(step0, step0);
    assert_eq!(step1, step1);
    assert_eq!(step2, step2);
    assert_eq!(step3, step3);

    assert_eq!(step1 - step0, STEP as isize);
    assert_eq!(step2 - step1, STEP as isize);
    assert_eq!(step3 - step2, STEP as isize);
    assert_eq!(step0 - step3, STEP as isize);

    assert_eq!(step0 - step1, -(STEP as isize));
    assert_eq!(step1 - step2, -(STEP as isize));
    assert_eq!(step2 - step3, -(STEP as isize));
    assert_eq!(step3 - step0, -(STEP as isize));

    assert!(step0 < step0 + 1);
    assert!(step1 < step1 + 1);
    assert!(step2 < step2 + 1);
    assert!(step3 < step3 + 1);

    assert!(step0 < step1);
    assert!(step1 < step2);
    assert!(step2 < step3);
    assert!(step3 < step0);

    assert!(step1 > step0);
    assert!(step2 > step1);
    assert!(step3 > step2);
    assert!(step0 > step3);
}

#[test]
fn test_multithread_counter_incr() {
    use std::sync::Arc;
    use std::thread;

    let counter = Arc::new(AtomicCounter::default());

    let handles: Vec<_> = (0..8).map(|_| {
        let counter = counter.clone();

        thread::spawn(move|| {
            for _ in 0..8000 {
                counter.incr();
            }
        })
    }).collect();

    for handle in handles {
        handle.join().unwrap();
    }

    assert_eq!(counter.fetch().unwrap(), Counter::new(64000));
}

#[test]
fn test_overflowed_counter_incr() {
    use std::sync::Arc;
    use std::thread;
    use std::usize;

    let counter_init = Counter::new(usize::MAX - 8000);
    let counter = Arc::new(AtomicCounter::new(counter_init));

    let handles: Vec<_> = (0..8).map(|_| {
        let counter = counter.clone();

        thread::spawn(move|| {
            for _ in 0..8000 {
                counter.incr();
            }
        })
    }).collect();

    for handle in handles {
        handle.join().unwrap();
    }

    let counter_end = counter.fetch().unwrap();
    assert_eq!(counter_end, counter_init + 64000);
    assert!(counter_end > counter_init);
}
