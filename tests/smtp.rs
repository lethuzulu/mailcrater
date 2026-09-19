use lettre::{
    AsyncSmtpTransport, AsyncTransport, Message, Tokio1Executor,
    message::{Attachment, MultiPart, SinglePart, header::ContentType},
};
use mailcrater::connection::MailServer;

async fn spawn_test_server() -> u16 {
    let server = MailServer::new("127.0.0.1:0")
        .await
        .expect("failed to bind test SMTP server");
    let port = server
        .local_addr()
        .expect("failed to read bound address")
        .port();

    tokio::spawn(async move {
        server.serve().await;
    });

    port
}

fn test_transport(port: u16) -> AsyncSmtpTransport<Tokio1Executor> {
    AsyncSmtpTransport::<Tokio1Executor>::builder_dangerous("127.0.0.1")
        .port(port)
        .build()
}

#[tokio::test]
async fn accepts_all_phase_1_message_shapes() {
    let port = spawn_test_server().await;
    let transport = test_transport(port);

    let plain_text = Message::builder()
        .from("no-reply@myapp.local".parse().unwrap())
        .to("bob@example.com".parse().unwrap())
        .subject("Reset your password")
        .header(ContentType::TEXT_PLAIN)
        .body(String::from("Click here to reset: https://myapp.local/reset"))
        .unwrap();
    let result = transport.send(plain_text).await;
    assert!(result.is_ok(), "plain text send failed: {result:?}");

    let attachment = Attachment::new("receipt.txt".to_string())
        .body(String::from("thanks for testing"), ContentType::TEXT_PLAIN);
    let html_with_attachment = Message::builder()
        .from("no-reply@myapp.local".parse().unwrap())
        .to("bob@example.com".parse().unwrap())
        .subject("Your receipt")
        .multipart(
            MultiPart::mixed()
                .singlepart(
                    SinglePart::builder()
                        .header(ContentType::TEXT_HTML)
                        .body(String::from("<p>Thanks for testing!</p>")),
                )
                .singlepart(attachment),
        )
        .unwrap();
    let result = transport.send(html_with_attachment).await;
    assert!(result.is_ok(), "html+attachment send failed: {result:?}");

    let multiple_recipients = Message::builder()
        .from("no-reply@myapp.local".parse().unwrap())
        .to("bob@example.com".parse().unwrap())
        .cc("admin@example.com".parse().unwrap())
        .subject("Reset your password")
        .header(ContentType::TEXT_PLAIN)
        .body(String::from("Click here to reset: https://myapp.local/reset"))
        .unwrap();
    let result = transport.send(multiple_recipients).await;
    assert!(result.is_ok(), "multi-recipient send failed: {result:?}");
}
