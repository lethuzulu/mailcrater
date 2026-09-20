use std::sync::atomic::{AtomicU32, Ordering};

use mailcrater::api;
use mailcrater::message::{NewAttachment, NewMessage};
use mailcrater::storage::Storage;
use reqwest::StatusCode;

static TEST_COUNTER: AtomicU32 = AtomicU32::new(0);

// Removes its directory on drop (including on test panic/assertion failure,
// since Drop still runs during unwinding) so repeated `cargo test` runs don't
// pile up directories under the OS temp dir.
struct TestDir(std::path::PathBuf);

impl Drop for TestDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

// Every test gets its own on-disk SQLite file (Storage doesn't support
// in-memory DBs) and its own OS-assigned port, so tests can run concurrently
// without touching each other's data or ports.
async fn spawn_test_server() -> (String, Storage, TestDir) {
    let n = TEST_COUNTER.fetch_add(1, Ordering::SeqCst);
    let dir = std::env::temp_dir().join(format!(
        "mailcrater_api_test_{}_{}",
        std::process::id(),
        n
    ));
    let _ = std::fs::remove_dir_all(&dir);

    let storage = Storage::new(&dir).await.expect("open storage");
    let app = api::app(storage.clone());

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind ephemeral port");
    let addr = listener.local_addr().expect("read local addr");

    tokio::spawn(async move {
        axum::serve(listener, app).await.expect("serve");
    });

    (format!("http://{addr}"), storage, TestDir(dir))
}

fn sample_message(subject: &str, from_addr: &str) -> NewMessage {
    NewMessage {
        from_addr: from_addr.to_string(),
        to_addrs: vec!["bob@example.com".to_string()],
        cc_addrs: vec![],
        subject: Some(subject.to_string()),
        body_text: Some("plain body".to_string()),
        body_html: None,
        raw_size: 5,
        raw_source: b"hello".to_vec(),
        attachments: vec![NewAttachment {
            filename: Some("receipt.txt".to_string()),
            content_type: Some("text/plain".to_string()),
            size: 5,
            data: b"hello".to_vec(),
        }],
    }
}

#[tokio::test]
async fn version_endpoint_returns_crate_version() {
    let (base_url, _storage, _dir) = spawn_test_server().await;

    let response = reqwest::get(format!("{base_url}/api/version"))
        .await
        .expect("request failed");

    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(response.text().await.unwrap(), env!("CARGO_PKG_VERSION"));
}

#[tokio::test]
async fn get_message_returns_404_for_unknown_id() {
    let (base_url, _storage, _dir) = spawn_test_server().await;

    let response = reqwest::get(format!("{base_url}/api/messages/does-not-exist"))
        .await
        .expect("request failed");

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn message_lifecycle_across_every_endpoint() {
    let (base_url, storage, _dir) = spawn_test_server().await;
    let client = reqwest::Client::new();

    let id = storage
        .insert_message(sample_message("Password reset", "alice@example.com"))
        .await
        .expect("seed message");

    // GET /api/messages/:id
    let detail: serde_json::Value = client
        .get(format!("{base_url}/api/messages/{id}"))
        .send()
        .await
        .expect("request failed")
        .json()
        .await
        .expect("parse json");
    assert_eq!(detail["subject"], "Password reset");
    assert_eq!(detail["from_addr"], "alice@example.com");
    let attachments = detail["attachments"].as_array().expect("attachments array");
    assert_eq!(attachments.len(), 1);
    let attachment_id = attachments[0]["id"].as_str().expect("attachment id").to_string();

    // GET /api/messages (list, no filter)
    let list: serde_json::Value = client
        .get(format!("{base_url}/api/messages"))
        .send()
        .await
        .expect("request failed")
        .json()
        .await
        .expect("parse json");
    assert_eq!(list.as_array().unwrap().len(), 1);

    // GET /api/messages?search=... (matching and non-matching)
    let matching: serde_json::Value = client
        .get(format!("{base_url}/api/messages?search=reset"))
        .send()
        .await
        .expect("request failed")
        .json()
        .await
        .expect("parse json");
    assert_eq!(matching.as_array().unwrap().len(), 1);

    let non_matching: serde_json::Value = client
        .get(format!("{base_url}/api/messages?search=nothing-matches-this"))
        .send()
        .await
        .expect("request failed")
        .json()
        .await
        .expect("parse json");
    assert_eq!(non_matching.as_array().unwrap().len(), 0);

    // GET /api/messages?limit=-1 -> 400
    let bad_limit = client
        .get(format!("{base_url}/api/messages?limit=-1"))
        .send()
        .await
        .expect("request failed");
    assert_eq!(bad_limit.status(), StatusCode::BAD_REQUEST);

    // GET /api/messages/:id/raw
    let raw = client
        .get(format!("{base_url}/api/messages/{id}/raw"))
        .send()
        .await
        .expect("request failed");
    assert_eq!(raw.status(), StatusCode::OK);
    assert_eq!(
        raw.headers().get("content-type").unwrap(),
        "message/rfc822"
    );
    assert_eq!(raw.bytes().await.unwrap().as_ref(), b"hello");

    // GET /api/messages/:id/attachments/:aid
    let attachment = client
        .get(format!(
            "{base_url}/api/messages/{id}/attachments/{attachment_id}"
        ))
        .send()
        .await
        .expect("request failed");
    assert_eq!(attachment.status(), StatusCode::OK);
    assert_eq!(
        attachment.headers().get("content-type").unwrap(),
        "text/plain"
    );
    assert_eq!(attachment.bytes().await.unwrap().as_ref(), b"hello");

    // wrong parent message id -> 404, even with a valid attachment id
    let wrong_parent = client
        .get(format!(
            "{base_url}/api/messages/some-other-id/attachments/{attachment_id}"
        ))
        .send()
        .await
        .expect("request failed");
    assert_eq!(wrong_parent.status(), StatusCode::NOT_FOUND);

    // DELETE /api/messages/:id
    let delete = client
        .delete(format!("{base_url}/api/messages/{id}"))
        .send()
        .await
        .expect("request failed");
    assert_eq!(delete.status(), StatusCode::NO_CONTENT);

    // deleted message is gone, and its attachment went with it (cascade)
    let after_delete = client
        .get(format!("{base_url}/api/messages/{id}"))
        .send()
        .await
        .expect("request failed");
    assert_eq!(after_delete.status(), StatusCode::NOT_FOUND);

    let attachment_after_delete = client
        .get(format!(
            "{base_url}/api/messages/{id}/attachments/{attachment_id}"
        ))
        .send()
        .await
        .expect("request failed");
    assert_eq!(attachment_after_delete.status(), StatusCode::NOT_FOUND);

    // deleting again -> 404, nothing left to delete
    let delete_again = client
        .delete(format!("{base_url}/api/messages/{id}"))
        .send()
        .await
        .expect("request failed");
    assert_eq!(delete_again.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn delete_all_clears_every_message() {
    let (base_url, storage, _dir) = spawn_test_server().await;
    let client = reqwest::Client::new();

    storage
        .insert_message(sample_message("First", "a@example.com"))
        .await
        .expect("seed message 1");
    storage
        .insert_message(sample_message("Second", "b@example.com"))
        .await
        .expect("seed message 2");

    let delete_all = client
        .delete(format!("{base_url}/api/messages"))
        .send()
        .await
        .expect("request failed");
    assert_eq!(delete_all.status(), StatusCode::NO_CONTENT);

    let list: serde_json::Value = client
        .get(format!("{base_url}/api/messages"))
        .send()
        .await
        .expect("request failed")
        .json()
        .await
        .expect("parse json");
    assert_eq!(list.as_array().unwrap().len(), 0);
}
