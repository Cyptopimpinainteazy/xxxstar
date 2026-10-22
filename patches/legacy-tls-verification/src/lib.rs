//! Real loopback TLS transport checks. No chain execution is claimed here.
#![cfg(test)]
use futures_util::{SinkExt, StreamExt};
use std::{
    io::{Read, Write},
    net::TcpListener,
    thread,
    time::{Duration, Instant},
};

struct Peer {
    port: u16,
    certificate: Vec<u8>,
    server: thread::JoinHandle<Result<(), String>>,
}

fn peer(websocket: bool) -> Peer {
    let signed = rcgen::generate_simple_self_signed(vec!["localhost".to_owned()]).unwrap();
    let certificate = signed.cert.der().to_vec();
    let identity = native_tls::Identity::from_pkcs8(
        signed.cert.pem().as_bytes(),
        signed.key_pair.serialize_pem().as_bytes(),
    )
    .unwrap();
    let acceptor = native_tls::TlsAcceptor::new(identity).unwrap();
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    listener.set_nonblocking(true).unwrap();
    let server = thread::spawn(move || {
        let deadline = Instant::now() + Duration::from_secs(10);
        let socket = loop {
            match listener.accept() {
                Ok((socket, _)) => break socket,
                Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                    if Instant::now() >= deadline {
                        return Err("no TLS connection arrived".into());
                    }
                    thread::sleep(Duration::from_millis(10));
                }
                Err(error) => return Err(error.to_string()),
            }
        };
        socket
            .set_read_timeout(Some(Duration::from_secs(3)))
            .unwrap();
        socket
            .set_write_timeout(Some(Duration::from_secs(3)))
            .unwrap();
        let mut stream = acceptor.accept(socket).map_err(|error| error.to_string())?;
        if websocket {
            let mut ws = tungstenite::accept(stream).map_err(|error| error.to_string())?;
            let message = ws.read().map_err(|error| error.to_string())?;
            if message.to_text().map_err(|error| error.to_string())? != "x3-tls-probe" {
                return Err("unexpected WebSocket payload".into());
            }
            ws.send(tungstenite::Message::Text("verified".into()))
                .map_err(|error| error.to_string())?;
        } else {
            let mut request = Vec::new();
            let mut buffer = [0; 512];
            while !request.ends_with(b"\r\n\r\n") {
                let size = stream
                    .read(&mut buffer)
                    .map_err(|error| error.to_string())?;
                if size == 0 || request.len() > 8192 {
                    return Err("incomplete HTTP request".into());
                }
                request.extend_from_slice(&buffer[..size]);
            }
            stream
                .write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 2\r\nConnection: close\r\n\r\nok")
                .map_err(|error| error.to_string())?;
        }
        Ok(())
    });
    Peer {
        port,
        certificate,
        server,
    }
}

fn assert_certificate_validation_error(error: &dyn std::error::Error) {
    let mut details = error.to_string();
    let mut source = error.source();
    while let Some(cause) = source {
        details.push_str(&cause.to_string());
        source = cause.source();
    }
    let lower = details.to_lowercase();
    assert!(
        [
            "certificate",
            "cert_verify",
            "wrong_principal",
            "hostname",
            "principal name"
        ]
        .iter()
        .any(|marker| lower.contains(marker)),
        "expected certificate validation failure, got {details}"
    );
}

#[test]
fn https_accepts_trusted_matching_certificate() {
    let peer = peer(false);
    let client = reqwest::blocking::Client::builder()
        .no_proxy()
        .timeout(Duration::from_secs(5))
        .add_root_certificate(reqwest::Certificate::from_der(&peer.certificate).unwrap())
        .build()
        .unwrap();
    let body = client
        .get(format!("https://localhost:{}/", peer.port))
        .send()
        .unwrap()
        .text()
        .unwrap();
    assert_eq!(body, "ok");
    peer.server.join().unwrap().unwrap();
}

#[test]
fn https_rejects_unknown_certificate() {
    let peer = peer(false);
    let client = reqwest::blocking::Client::builder()
        .no_proxy()
        .timeout(Duration::from_secs(5))
        .build()
        .unwrap();
    let error = client
        .get(format!("https://localhost:{}/", peer.port))
        .send()
        .expect_err("certificate validation must reject the peer");
    assert!(error.is_connect());
    assert_certificate_validation_error(&error);
    assert!(peer.server.join().unwrap().is_err());
}

#[test]
fn https_rejects_trusted_certificate_for_wrong_hostname() {
    let peer = peer(false);
    let client = reqwest::blocking::Client::builder()
        .no_proxy()
        .timeout(Duration::from_secs(5))
        .add_root_certificate(reqwest::Certificate::from_der(&peer.certificate).unwrap())
        .build()
        .unwrap();
    let error = client
        .get(format!("https://127.0.0.1:{}/", peer.port))
        .send()
        .expect_err("certificate validation must reject the peer");
    assert!(error.is_connect());
    assert_certificate_validation_error(&error);
    assert!(peer.server.join().unwrap().is_err());
}

fn websocket_roundtrip(trusted: bool, host: &str, should_succeed: bool) {
    let peer = peer(true);
    let mut builder = native_tls::TlsConnector::builder();
    if trusted {
        builder.add_root_certificate(native_tls::Certificate::from_der(&peer.certificate).unwrap());
    }
    let connector = tokio_tungstenite::Connector::NativeTls(builder.build().unwrap());
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    runtime.block_on(async {
        let result = tokio::time::timeout(
            Duration::from_secs(5),
            tokio_tungstenite::connect_async_tls_with_config(
                format!("wss://{host}:{}/", peer.port),
                None,
                false,
                Some(connector),
            ),
        )
        .await
        .unwrap();
        if should_succeed {
            let (mut ws, _) = result.unwrap();
            ws.send(tungstenite::Message::Text("x3-tls-probe".into()))
                .await
                .unwrap();
            let message = tokio::time::timeout(Duration::from_secs(5), ws.next())
                .await
                .unwrap()
                .unwrap()
                .unwrap();
            assert_eq!(message.into_text().unwrap(), "verified");
        } else {
            let error = result.expect_err("TLS must reject the certificate");
            assert!(matches!(&error, tungstenite::Error::Tls(_)));
            assert_certificate_validation_error(&error);
        }
    });
    let server = peer.server.join().unwrap();
    if should_succeed {
        server.unwrap();
    } else {
        assert!(server.is_err());
    }
}

#[test]
fn wss_accepts_trusted_matching_certificate_and_exchanges_frames() {
    websocket_roundtrip(true, "localhost", true);
}
#[test]
fn wss_rejects_unknown_certificate() {
    websocket_roundtrip(false, "localhost", false);
}

#[test]
fn wss_rejects_trusted_certificate_for_wrong_hostname() {
    websocket_roundtrip(true, "127.0.0.1", false);
}
