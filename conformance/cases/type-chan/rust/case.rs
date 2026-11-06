use std::sync::mpsc::Sender;

/// Sends n values on a channel.
pub fn pipe(ch: Sender<String>, n: i64) {
    let mut i: i64 = 0;
    while i < n {
        ch.send("v".to_string()).unwrap();
        i += 1;
    }
}
