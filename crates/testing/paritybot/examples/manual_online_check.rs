//! `docs/MANUAL-VERIFICATION-M1.md`'s own procedure (b): a small, interactive-only
//! example that performs a real Microsoft/Xbox device-code OAuth login and then
//! attempts to join `<host>:<port>`. Deliberately never wired into `cargo nextest
//! run`'s default test set (Constraints (f)) — an `examples/` target, run only by
//! explicit human invocation, exactly as that document names it.
//!
//! Usage: `cargo run -p rc-paritybot --example manual_online_check -- <host> <port>
//! <email>`
//!
//! `<email>` is used as azalea's own auth cache key (commonly the account's email
//! address) — this example never reads, stores, or transmits a password: the OAuth
//! device-code flow prints a short code and a `https://microsoft.com/link` URL to
//! this terminal, which you open and complete in a real browser.

use std::time::Duration;

use azalea::prelude::*;

async fn handle(_bot: Client, event: Event, _state: azalea::NoState) {
    match event {
        Event::Login => println!("manual_online_check: Event::Login"),
        Event::Spawn => println!("manual_online_check: Event::Spawn — join succeeded"),
        Event::Disconnect(reason) => {
            println!(
                "manual_online_check: Event::Disconnect: {}",
                reason.map(|r| r.to_string()).unwrap_or_default()
            );
        }
        _ => {}
    }
}

#[tokio::main]
async fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let [host, port, email] = match <[String; 3]>::try_from(args) {
        Ok(a) => a,
        Err(_) => {
            eprintln!("usage: manual_online_check <host> <port> <email>");
            std::process::exit(1);
        }
    };

    println!(
        "manual_online_check: starting Microsoft device-code login for {email:?} -- \
         follow the printed instructions in this terminal"
    );
    let account = match azalea::account::Account::microsoft(&email).await {
        Ok(account) => account,
        Err(err) => {
            eprintln!("manual_online_check: Microsoft login failed: {err}");
            std::process::exit(1);
        }
    };

    let address = format!("{host}:{port}");
    println!("manual_online_check: logged in, connecting to {address}");

    let _ = tokio::time::timeout(
        Duration::from_secs(60),
        ClientBuilder::new()
            .set_handler(handle)
            .start(account, address.as_str()),
    )
    .await;
}
