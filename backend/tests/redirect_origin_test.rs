#[path = "../vendor/ccxt-core-0.1.5/src/http_client/redirect.rs"]
mod redirect;

use reqwest::Url;

fn allowed(initial: &str, destination: &str) -> bool {
    redirect::redirect_allowed(
        &[Url::parse(initial).unwrap()],
        &Url::parse(destination).unwrap(),
    )
}

#[test]
fn accepts_only_the_same_origin() {
    assert!(allowed(
        "http://example.test:80/start",
        "http://example.test/next"
    ));
    assert!(!allowed(
        "http://example.test:80/start",
        "http://example.test:81/next"
    ));
    assert!(!allowed(
        "http://example.test/start",
        "http://other.test/next"
    ));
}

#[test]
fn rejects_downgrade_and_userinfo() {
    assert!(!allowed(
        "https://example.test/start",
        "http://example.test/next"
    ));
    assert!(!allowed(
        "https://user@example.test/start",
        "https://example.test/next"
    ));
    assert!(!allowed(
        "https://example.test/start",
        "https://user@example.test/next"
    ));
}

#[test]
fn rejects_empty_history_and_more_than_ten_redirects() {
    let url = Url::parse("https://example.test/start").unwrap();
    assert!(!redirect::redirect_allowed(&[], &url));
    assert!(redirect::redirect_allowed(&vec![url.clone(); 10], &url));
    assert!(!redirect::redirect_allowed(&vec![url.clone(); 11], &url));
}
