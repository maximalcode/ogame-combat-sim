use std::{
    ffi::OsString,
    io,
    net::{SocketAddr, TcpListener, TcpStream},
    process::{Command, Stdio},
    thread,
    time::{Duration, Instant},
};

fn free_port() -> u16 {
    TcpListener::bind(SocketAddr::from(([127, 0, 0, 1], 0)))
        .expect("reserve an ephemeral port")
        .local_addr()
        .expect("read ephemeral port")
        .port()
}

fn start_and_probe(port: OsString, max_simulations: OsString, expected_port: u16) {
    let mut process = Command::new(env!("CARGO_BIN_EXE_combat-api"))
        .env_clear()
        .env("PORT", port)
        .env("MAX_SIMULATIONS", max_simulations)
        .env("MAX_CONCURRENT_SIMULATIONS", "1")
        .env("SHUTDOWN_GRACE_SECONDS", "1")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("start combat-api");

    let address = SocketAddr::from(([127, 0, 0, 1], expected_port));
    let deadline = Instant::now() + Duration::from_secs(2);
    loop {
        if let Some(status) = process.try_wait().expect("poll combat-api") {
            panic!("combat-api rejected a legacy setting: {status}");
        }
        match TcpStream::connect_timeout(&address, Duration::from_millis(50)) {
            Ok(_) => break,
            Err(error) if Instant::now() < deadline => {
                if error.kind() != io::ErrorKind::ConnectionRefused {
                    thread::yield_now();
                }
                thread::sleep(Duration::from_millis(10));
            }
            Err(error) => panic!("combat-api did not start: {error}"),
        }
    }

    process.kill().expect("stop combat-api");
    process.wait().expect("reap combat-api");
}

#[test]
fn legacy_port_and_simulation_cap_values_fall_back_to_defaults() {
    for value in ["invalid", "", "70000"] {
        start_and_probe(value.into(), "1000".into(), 3000);
    }
    #[cfg(unix)]
    start_and_probe(OsString::from_vec(vec![0xff]), "1000".into(), 3000);

    for value in ["invalid", "", "4294967296"] {
        let port = free_port();
        start_and_probe(port.to_string().into(), value.into(), port);
    }
    #[cfg(unix)]
    {
        let port = free_port();
        start_and_probe(
            port.to_string().into(),
            OsString::from_vec(vec![0xff]),
            port,
        );
    }
}

#[cfg(unix)]
use std::os::unix::ffi::OsStringExt;
