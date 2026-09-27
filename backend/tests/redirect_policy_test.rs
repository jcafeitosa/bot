use ccxt_core::http_client::{HttpClient, HttpConfig};
use ccxt_core::retry_strategy::RetryConfig;
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use tokio::sync::oneshot;

async fn serve_once(listener: TcpListener, response: String) {
    let (mut stream, _) = listener.accept().await.unwrap();
    let mut request = [0_u8; 1024];
    assert!(stream.read(&mut request).await.unwrap() > 0);
    stream.write_all(response.as_bytes()).await.unwrap();
    stream.shutdown().await.unwrap();
}

fn response(status: &str, body: &str, location: Option<&str>) -> String {
    let location = location
        .map(|url| format!("Location: {url}\r\n"))
        .unwrap_or_default();
    format!(
        "HTTP/1.1 {status}\r\nContent-Type: application/json\r\nContent-Length: {}\r\n{location}Connection: close\r\n\r\n{body}",
        body.len()
    )
}

fn client() -> HttpClient {
    HttpClient::new(HttpConfig {
        retry_config: Some(RetryConfig {
            max_retries: 0,
            ..RetryConfig::default()
        }),
        ..HttpConfig::default()
    })
    .unwrap()
}

#[tokio::test]
async fn refuses_cross_origin_redirect_before_connecting_to_destination() {
    tokio::time::timeout(Duration::from_secs(4), async {
        let source = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let source_addr = source.local_addr().unwrap();
        let destination = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let destination_addr = destination.local_addr().unwrap();
        let source_task = tokio::spawn(serve_once(
            source,
            response(
                "302 Found",
                "",
                Some(&format!("http://{destination_addr}/stolen")),
            ),
        ));
        let (done_tx, mut done_rx) = oneshot::channel::<()>();
        let destination_task = tokio::spawn(async move {
            tokio::select! {
                biased;
                accepted = destination.accept() => {
                    let (mut stream, _) = accepted.unwrap();
                    let mut request = [0_u8; 1024];
                    assert!(stream.read(&mut request).await.unwrap() > 0);
                    stream
                        .write_all(response("200 OK", r#"{"stolen":true}"#, None).as_bytes())
                        .await
                        .unwrap();
                    true
                }
                _ = &mut done_rx => false,
            }
        });

        let result = client()
            .get(&format!("http://{source_addr}/start"), None)
            .await;
        let _ = done_tx.send(());
        source_task.await.unwrap();
        let destination_contacted = destination_task.await.unwrap();
        assert!(
            result.is_err(),
            "cross-origin redirect must surface as error"
        );
        assert!(
            !destination_contacted,
            "redirect destination received a connection"
        );
    })
    .await
    .expect("local redirect test timed out");
}

#[tokio::test]
async fn follows_redirect_within_initial_origin() {
    tokio::time::timeout(Duration::from_secs(4), async {
        let source = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let source_addr = source.local_addr().unwrap();
        let source_task = tokio::spawn(async move {
            let (mut first, _) = source.accept().await.unwrap();
            let mut request = [0_u8; 1024];
            assert!(first.read(&mut request).await.unwrap() > 0);
            first
                .write_all(
                    response("302 Found", "", Some(&format!("http://{source_addr}/ok"))).as_bytes(),
                )
                .await
                .unwrap();
            first.shutdown().await.unwrap();

            let (mut second, _) = source.accept().await.unwrap();
            assert!(second.read(&mut request).await.unwrap() > 0);
            second
                .write_all(response("200 OK", r#"{"ok":true}"#, None).as_bytes())
                .await
                .unwrap();
            second.shutdown().await.unwrap();
        });

        let result = client()
            .get(&format!("http://{source_addr}/start"), None)
            .await
            .unwrap();
        source_task.await.unwrap();
        assert_eq!(result["ok"], true);
    })
    .await
    .expect("local redirect test timed out");
}
