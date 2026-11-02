//! Synthetic solver states: even a cyclic parent walk must reach the budget callback.
use super::*;

#[test]
fn cyclic_bk_walks_stop_within_ten_seconds() {
    let (send, receive) = std::sync::mpsc::channel();
    let worker = std::thread::spawn(move || {
        for augment in [true, false] {
            let mut g = Graph::with_capacity(3, 2);
            g.add_edge(0, 1, 1.0, 1.0);
            g.add_edge(1, 2, 1.0, 1.0);
            g.add_tweights(0, 1.0, 0.0);
            g.add_tweights(2, 0.0, 1.0);
            // Deliberately model a stale search tree: 0's parent leads to 1 and back to 0.
            // The test must terminate even if a future numerical bug creates this state.
            g.parent[0] = 0;
            g.parent[1] = 1;
            g.parent[2] = if augment { TERMINAL } else { ORPHAN };
            let asks = std::cell::Cell::new(0);
            let limited = || {
                asks.set(asks.get() + 1);
                asks.get() > 2
            };
            let mut steps = Steps::new(&limited);
            let mut orphans = VecDeque::new();
            let result = if augment { g.augment(2, &mut orphans, &mut steps) } else { g.adopt(2, 1, &mut VecDeque::new(), &mut orphans, &mut steps) };
            assert_eq!(result, None, "cyclic walk must spend its budget (augment={augment})");
            assert_eq!(asks.get(), 3);
        }
        send.send(()).unwrap();
    });
    receive.recv_timeout(std::time::Duration::from_secs(10)).expect("cyclic BK walk evaded the step budget");
    worker.join().unwrap();
}
