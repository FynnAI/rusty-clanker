//! Regression test for the stdin-shutdown protocol's EOF semantics (main.rs's own
//! documented contract): closing the server's stdin must NOT shut it down — only an
//! explicit `shutdown` line may. A dropped `oneshot::Sender` resolves its receiver
//! with `Err`, and an arm that ignores that `Result` turns EOF into an accidental
//! clean shutdown — exactly the observed field failure this test pins.

use std::io::Write;
use std::net::TcpStream;
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

fn spawn_server(port: u16) -> Child {
    Command::new(env!("CARGO_BIN_EXE_rusty-clanker-server"))
        .args(["--bind", &format!("127.0.0.1:{port}"), "--offline"])
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("server binary spawns")
}

fn wait_for_listen(port: u16, deadline: Duration) -> bool {
    let start = Instant::now();
    while start.elapsed() < deadline {
        if TcpStream::connect(("127.0.0.1", port)).is_ok() {
            return true;
        }
        std::thread::sleep(Duration::from_millis(100));
    }
    false
}

#[test]
fn stdin_eof_does_not_shut_the_server_down() {
    let port = 25599;
    let mut child = spawn_server(port);
    assert!(
        wait_for_listen(port, Duration::from_secs(20)),
        "server never listened"
    );

    // Close stdin: the reader task sees EOF and must simply retire.
    drop(child.stdin.take());

    // The observed field failure shut down within milliseconds; give it ample time
    // to misbehave, then prove the listener is still alive.
    std::thread::sleep(Duration::from_secs(3));
    assert!(
        child.try_wait().expect("try_wait works").is_none(),
        "server exited after stdin EOF — EOF must not trigger shutdown"
    );
    assert!(
        TcpStream::connect(("127.0.0.1", port)).is_ok(),
        "server no longer accepting after stdin EOF"
    );

    let _ = child.kill();
    let _ = child.wait();
}

#[test]
fn explicit_shutdown_line_still_stops_the_server() {
    let port = 25598;
    let mut child = spawn_server(port);
    assert!(
        wait_for_listen(port, Duration::from_secs(20)),
        "server never listened"
    );

    let mut stdin = child.stdin.take().expect("piped stdin");
    stdin.write_all(b"shutdown\n").expect("write shutdown line");
    stdin.flush().expect("flush");
    drop(stdin);

    let start = Instant::now();
    let exited_cleanly = loop {
        match child.try_wait().expect("try_wait works") {
            Some(status) => break status.success(),
            None if start.elapsed() > Duration::from_secs(30) => break false,
            None => std::thread::sleep(Duration::from_millis(200)),
        }
    };
    if !exited_cleanly {
        let _ = child.kill();
        let _ = child.wait();
    }
    assert!(
        exited_cleanly,
        "explicit `shutdown` line must still stop the server cleanly"
    );
}
