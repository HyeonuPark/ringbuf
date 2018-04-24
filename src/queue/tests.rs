use super::*;

use std::thread;
use std::sync::mpsc;
use std::time::Duration;

use rand::{thread_rng, Rng};

use sequence::{Owned, Shared};

#[test]
fn test_spining_spsc() {
    const COUNT: u32 = 640;
    let (mut tx, mut rx) = bounded::queue::<Owned, Owned, u32>(16);
    let (done1, wait1) = mpsc::channel();
    let (done2, wait2) = mpsc::channel();

    let tx = thread::spawn(move|| {
        for i in 0..COUNT {
            loop {
                if let Ok(()) = tx.try_send(i) {
                    break;
                }
            }
        }

        done1.send(()).unwrap();
    });

    let rx = thread::spawn(move|| {
        for i in 0..COUNT {
            loop {
                if let Some(recv) = rx.try_recv() {
                    assert_eq!(i, recv);
                    break;
                }
            }
        }

        done2.send(()).unwrap();
    });

    thread::sleep(Duration::from_secs(5));
    wait1.try_recv().expect("Sender thread takes too long");
    wait2.try_recv().expect("Receiver thread takes too long");

    tx.join().expect("Sender thread panicked");
    rx.join().expect("Receiver thread panicked");
}

#[test]
fn test_spninning_mpmc() {
    const COUNT: u32 = 320;
    const THREADS: u32 = 4;

    let (tx, mut rx) = bounded::queue::<Shared, Shared, u32>(32);
    let (done1, wait1) = mpsc::channel();
    let (done2, wait2) = mpsc::channel();

    let tx_handles: Vec<_> = (0..THREADS).map(|_n| {
        let mut tx = tx.clone();
        let done = done1.clone();
        thread::spawn(move|| {
            let mut rng = thread_rng();
            let mut acc = 0u64;

            for _i in 0..COUNT {
                let num = rng.gen_range(0u32, 256);

                loop {
                    if let Ok(()) = tx.try_send(num) {
                        // println!("sent: {} - {}", _n, _i);
                        acc += num as u64;
                        break;
                    }
                }
            }

            done.send(()).unwrap();
            acc
        })
    }).collect();

    let rx_handles: Vec<_> = (0..THREADS).map(|_n| {
        let mut rx = rx.clone();
        let done = done2.clone();
        thread::spawn(move|| {
            let mut acc = 0u64;

            for _i in 0..COUNT {
                loop {
                    if let Some(v) = rx.try_recv() {
                        // println!("recv: {} - {}", _n, _i);
                        acc += v as u64;
                        break;
                    }
                }
            }

            done.send(()).unwrap();
            acc
        })
    }).collect();

    drop(tx);
    let start = ::std::time::Instant::now();

    // thread::sleep(Duration::from_secs(5));
    // for _ in 0..THREADS {
    //     wait1.try_recv().expect("Sender thread takes too long");
    //     wait2.try_recv().expect("Receiver thread takes too long");
    // }

    let tx_acc: u64 = tx_handles.into_iter()
        .map(|h| h.join().expect("Sender thread panicked"))
        .sum();

    let rx_acc: u64 = rx_handles.into_iter()
        .map(|h| h.join().expect("Receiver thread panicked"))
        .sum();

    assert_eq!(tx_acc, rx_acc);
    assert_eq!(rx.try_recv(), None);

    println!("spinning_mpmc takes {:?}", ::std::time::Instant::now() - start);
}

#[test]
fn test_growable_spinning_spsc() {
    const COUNT: u32 = 640;
    let (mut tx, mut rx) = growable::queue::<Owned, Owned, u32>();
    let (done1, wait1) = mpsc::channel();
    let (done2, wait2) = mpsc::channel();

    let tx = thread::spawn(move|| {
        for i in 0..COUNT {
            tx.send(i);
        }
        done1.send(()).unwrap();
    });

    let rx = thread::spawn(move|| {
        for i in 0..COUNT {
            loop {
                if let Some(recv) = rx.try_recv() {
                    assert_eq!(i, recv);
                    break;
                }
            }
        }
        done2.send(()).unwrap();
    });

    thread::sleep(Duration::from_secs(5));
    wait1.try_recv().expect("Sender thread takes too long");
    wait2.try_recv().expect("Receiver thread takes too long");

    tx.join().expect("Sender thread panicked");
    rx.join().expect("Receiver thread panicked");
}

#[test]
fn test_growable_spninning_mpmc() {
    const COUNT: u32 = 320;
    const THREADS: u32 = 4;

    let (tx, mut rx) = growable::queue::<Shared, Shared, u32>();
    let (done1, wait1) = mpsc::channel();
    let (done2, wait2) = mpsc::channel();

    let tx_handles: Vec<_> = (0..4).map(|_n| {
        let mut tx = tx.clone();
        let done = done1.clone();
        thread::spawn(move|| {
            let mut rng = thread_rng();
            let mut acc = 0u64;

            for _i in 0..COUNT {
                let num = rng.gen_range(0u32, 256);
                tx.send(num);
                acc += num as u64;
            }

            done.send(()).unwrap();
            acc
        })
    }).collect();

    let rx_handles: Vec<_> = (0..4).map(|_n| {
        let mut rx = rx.clone();
        let done = done2.clone();
        thread::spawn(move|| {
            let mut acc = 0u64;

            for _i in 0..COUNT {
                loop {
                    if let Some(v) = rx.try_recv() {
                        // println!("recv: {} - {}", _n, _i);
                        acc += v as u64;
                        break;
                    }
                }
            }

            done.send(()).unwrap();
            acc
        })
    }).collect();

    drop(tx);

    thread::sleep(Duration::from_secs(5));
    for _ in 0..THREADS {
        wait1.try_recv().expect("Sender thread takes too long");
        wait2.try_recv().expect("Receiver thread takes too long");
    }

    let tx_acc: u64 = tx_handles.into_iter()
        .map(|h| h.join().expect("Sender thread panicked"))
        .sum();

    let rx_acc: u64 = rx_handles.into_iter()
        .map(|h| h.join().expect("Receiver thread panicked"))
        .sum();

    assert_eq!(tx_acc, rx_acc);
    assert_eq!(rx.try_recv(), None);
}
