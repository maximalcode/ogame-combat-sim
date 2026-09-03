use super::{ReportClient, ReportError, ReportId};
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

#[tokio::test]
async fn client_imports_from_a_stub_and_redacts_provider_failures() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("http://{}/", listener.local_addr().unwrap());
    let server = tokio::spawn(async move {
        let success = r#"{"RESULT_CODE":1000,"RESULT_DATA":{"generic":{"failed_ships":false},"details":{"ships":[]}}}"#;
        let replies = vec![
            format!("HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{success}", success.len()),
            "HTTP/1.1 400 Bad\r\nContent-Length: 0\r\n\r\n".to_owned(),
            "HTTP/1.1 200 OK\r\nConnection: close\r\n\r\n{\"error\":{\"message\":\"private-token\"}}".to_owned(),
            "HTTP/1.1 200 OK\r\nConnection: close\r\n\r\nprivate-token".to_owned(),
            "HTTP/1.1 200 OK\r\nContent-Length: 9999999\r\n\r\n".to_owned(),
            format!("HTTP/1.1 200 OK\r\nConnection: close\r\n\r\n{}", "x".repeat(super::MAX_REPORT_BYTES + 1)),
            "HTTP/1.1 302 Found\r\nLocation: http://127.0.0.1:1/private-token\r\nContent-Length: 0\r\n\r\n".to_owned(),
            String::new(),
            "HTTP/1.1 429 Too Many Requests\r\nRetry-After: private-token\r\nContent-Length: 0\r\n\r\n".to_owned(),
            format!("HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{success}", success.len()),
            format!("HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{success}", success.len()),
        ];
        for reply in replies {
            let (mut socket, _) = listener.accept().await.unwrap();
            let mut request = vec![0; 4096];
            let size = socket.read(&mut request).await.unwrap();
            let request = std::str::from_utf8(&request[..size]).unwrap();
            assert!(request.starts_with(
                "GET /v1/report/sr-en-1-0000000000000000000000000000000000000000/1 HTTP/1.1"
            ));
            if reply.is_empty() {
                tokio::time::sleep(Duration::from_millis(1100)).await;
            }
            // Oversized responses may be rejected before the server finishes writing.
            let _ = socket.write_all(reply.as_bytes()).await;
        }
    });
    let client = ReportClient::with_endpoint(&url, Duration::from_secs(1)).unwrap();
    let id = ReportId::parse("sr-en-1-0000000000000000000000000000000000000000").unwrap();
    assert_eq!(
        client.fetch(&id).await.unwrap().report_kind,
        super::ReportKind::Espionage
    );
    let error = client.fetch(&id).await.unwrap_err();
    assert_eq!(error, ReportError::HttpStatus(400));
    assert!(!format!("{error:?} {error}").contains("private-token"));
    for expected in [
        ReportError::Provider,
        ReportError::Malformed,
        ReportError::TooLarge,
        ReportError::TooLarge,
        ReportError::HttpStatus(302),
        ReportError::Timeout,
        ReportError::RateLimited {
            retry_after_seconds: 60,
        },
    ] {
        assert_eq!(client.fetch(&id).await.unwrap_err(), expected);
    }
    assert!(client.fetch(&id).await.is_ok());
    // Separate client instances and concurrent callers share the same quota.
    let second = ReportClient::with_endpoint(&url, Duration::from_secs(2)).unwrap();
    let (first, second) = tokio::join!(client.fetch(&id), second.fetch(&id));
    for result in [first, second] {
        assert!(matches!(
            result,
            Err(ReportError::RateLimited {
                retry_after_seconds: 1..=60
            })
        ));
    }
    tokio::time::pause();
    tokio::time::advance(Duration::from_secs(60)).await;
    tokio::time::resume();
    assert!(client.fetch(&id).await.is_ok());
    server.await.unwrap();
}
