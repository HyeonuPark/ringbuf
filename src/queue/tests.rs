
use std::thread;

use sequence::{Owned};
use queue;

#[test]
fn test_spinning_spsc() {
    const COUNT: u32 = 640000;
    let (mut tx, mut rx) = queue::create::<Owned<u32>, Owned<u32>>(16);

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

    loop {
        if let Ok(res) = rx.try_recv() {
            assert_eq!(res, None);
            break;
        }
    }

    handle.join().unwrap();
}
