
use std::thread;

use rand::{Rng, thread_rng};

use sequence::{Owned, Competitive};
use channel::channel;

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
fn test_spinning_spsc() {
    let (mut tx, mut rx) = channel::<Owned, Owned, usize>(SIZE);

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
                assert_eq!(num, i);
                break;
            }
        }
    }

    handle.join().unwrap();
}

#[cfg(not(feature = "ci"))]
#[test]
fn test_spinning_mpmc() {
    let (tx, rx) = channel::<Competitive, Competitive, u64>(SIZE);

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
                            acc += num;
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
fn test_sync_spsc() {
    let (mut tx, mut rx) = channel::<Owned, Owned, usize>(SIZE);

    let handle = thread::spawn(move|| {
        for i in 0..COUNT {
            tx.sync_send(i);
        }
    });

    for i in 0..COUNT {
        assert_eq!(rx.sync_recv(), i);
    }

    handle.join().unwrap();
}

#[test]
fn test_sync_mpmc() {
    let (tx, rx) = channel::<Competitive, Competitive, u64>(SIZE);

    let senders: Vec<_> = (0..THREADS)
        .map(|_| {
            let mut tx = tx.clone();
            thread::spawn(move|| {
                let mut rng = thread_rng();
                let mut acc = 0u64;

                for _i in 0..COUNT {
                    let num = rng.gen_range(0u64, 1024);
                    acc += num;
                    tx.sync_send(num);
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
                    acc += rx.sync_recv();
                }

                acc
            })
        })
        .collect();

    let tx_sum: u64 = senders.into_iter().map(|h| h.join().unwrap()).sum();
    let rx_sum: u64 = receivers.into_iter().map(|h| h.join().unwrap()).sum();

    assert_eq!(tx_sum, rx_sum);
}
