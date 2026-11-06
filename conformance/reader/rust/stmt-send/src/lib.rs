use std::sync::mpsc::Sender;

/// Sends n and n+1.
pub fn emit(ch: Sender<i64>, n: i64) {
    ch.send(n).unwrap();
    ch.send(n + 1).unwrap();
}
