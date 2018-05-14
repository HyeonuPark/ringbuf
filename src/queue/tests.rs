
use std::thread;
use std::time::Duration;

use rand::{Rng, thread_rng};

use sequence::{Owned, Competitive};
use queue::{bounded, unbounded};

#[cfg(not(feature = "ci"))]
const COUNT: usize = 64000;
#[cfg(not(feature = "ci"))]
const SIZE: usize = 4;
#[cfg(not(feature = "ci"))]
const THREADS: usize = 2;

#[cfg(feature = "ci")]
const COUNT: usize = 6400000;
#[cfg(feature = "ci")]
const SIZE: usize = 128;
#[cfg(feature = "ci")]
const THREADS: usize = 4;

#[test]
fn test_spinning_bounded_spsc() {
    let (mut tx, mut rx) = bounded::queue::<Owned, Owned, usize>(SIZE);

    let handle = thread::spawn(move|| {
        for i in 0..COUNT {
            loop {
                if let Ok(()) = tx.try_send(i) {
                    break;
                }
            }
        }
    });

    for i in 0..COUNT {
        loop {
            if let Ok(num) = rx.try_recv() {
                assert_eq!(num, Some(i));
                break;
            }
        }
    }

    handle.join().unwrap();
    thread::sleep(Duration::from_millis(10)); // to ensure atomic closure is propagated
    assert_eq!(rx.try_recv(), Ok(None));
}

#[cfg(not(feature = "ci"))]
#[test]
fn test_spinning_bounded_mpmc() {
    let (tx, rx) = bounded::queue::<Competitive, Competitive, u64>(SIZE);

    let senders: Vec<_> = (0..THREADS)
        .map(|_| {
            let mut tx = tx.clone();
            thread::spawn(move|| {
                let mut rng = thread_rng();
                let mut acc = 0u64;

                for _ in 0..COUNT {
                    let num = rng.gen_range(0u64, 1024);
                    acc += num;

                    loop {
                        if let Ok(()) = tx.try_send(num) {
                            break;
                        }
                    }
                }

                acc
            })
        })
        .collect();

    let receivers: Vec<_> = (0..THREADS)
        .map(|_| {
            let mut rx = rx.clone();
            thread::spawn(move|| {
                let mut acc = 0u64;

                for _ in 0..COUNT {
                    loop {
                        if let Ok(num) = rx.try_recv() {
                            acc += num.unwrap();
                            break;
                        }
                    }
                }

                acc
            })
        })
        .collect();

    let tx_sum: u64 = senders.into_iter().map(|h| h.join().unwrap()).sum();
    let rx_sum: u64 = receivers.into_iter().map(|h| h.join().unwrap()).sum();

    assert_eq!(tx_sum, rx_sum);
}

#[test]
fn test_spinning_unbounded_spsc() {
    let (mut tx, mut rx) = unbounded::queue::<Owned, Owned, usize>();

    let handle = thread::spawn(move|| {
        for i in 0..COUNT {
            println!("SEND INIT {}", i);
            loop {
                if let Ok(()) = tx.try_send(i) {
                    break;
                }
            }
            println!("SEND DONE {}", i);
        }
    });

    for i in 0..COUNT {
        // println!("RECV INIT {}", i);
        loop {
            if let Ok(num) = rx.try_recv() {
                assert_eq!(num, Some(i));
                break;
            }
        }
        // println!("RECV DONE {}", i);
    }

    handle.join().unwrap();
    thread::sleep(Duration::from_millis(10)); // to ensure atomic closure is propagated
    assert_eq!(rx.try_recv(), Ok(None));
}

#[cfg(not(feature = "ci"))]
#[test]
fn test_spinning_unbounded_mpmc() {
    let (tx, rx) = unbounded::queue::<Competitive, Competitive, u64>();

    let senders: Vec<_> = (0..THREADS)
        .map(|_| {
            let mut tx = tx.clone();
            thread::spawn(move|| {
                let mut rng = thread_rng();
                let mut acc = 0u64;

                for _ in 0..COUNT {
                    let num = rng.gen_range(0u64, 1024);
                    acc += num;

                    loop {
                        if let Ok(()) = tx.try_send(num) {
                            break;
                        }
                    }
                }

                acc
            })
        })
        .collect();

    let receivers: Vec<_> = (0..THREADS)
        .map(|_| {
            let mut rx = rx.clone();
            thread::spawn(move|| {
                let mut acc = 0u64;

                for _ in 0..COUNT {
                    loop {
                        if let Ok(num) = rx.try_recv() {
                            acc += num.unwrap();
                            break;
                        }
                    }
                }

                acc
            })
        })
        .collect();

    let tx_sum: u64 = senders.into_iter().map(|h| h.join().unwrap()).sum();
    let rx_sum: u64 = receivers.into_iter().map(|h| h.join().unwrap()).sum();

    assert_eq!(tx_sum, rx_sum);
}
