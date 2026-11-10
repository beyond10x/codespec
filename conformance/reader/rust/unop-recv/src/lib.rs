use std::rc::Rc;
use std::sync::mpsc::{Receiver, SyncSender};

/// Sends n and receives it back: the channel is both of its ends, made by the caller with room for
/// 64 values. `try_send` and `try_recv` panic on a full and an empty channel, where Go's only
/// goroutine would deadlock.
pub fn echo(ch: Rc<(SyncSender<i64>, Receiver<i64>)>, n: i64) -> i64 {
    ch.0.try_send(n).expect("a send on a full channel");
    ch.1.try_recv().expect("a receive from an empty channel")
}
