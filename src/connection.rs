use std::net::SocketAddr;
use std::sync::Arc;


use crate::handler::MailHandler;
use crate::message::MailMessage;
use anyhow::Result;
use mailin::{Action, Response, SessionBuilder};
use tokio::io::{AsyncBufReadExt, AsyncWrite};
use tokio::sync::mpsc::Sender;
use tokio::{
    io::{AsyncWriteExt, BufReader},
    net::{TcpListener, TcpStream, ToSocketAddrs},
};

// #[derive(Debug)]
pub struct MailServer {
    inner: TcpListener,
    session_builder: Arc<SessionBuilder>,
    mail_handler: MailHandler,
}

impl MailServer {
    pub async fn new(
        address: impl ToSocketAddrs,
        sender: Sender<MailMessage>,
        max_message_size: usize,
    ) -> Result<Self> {
        let inner = TcpListener::bind(address).await?;
        let session_builder = Arc::new(SessionBuilder::new("mailcrater"));
        let mail_handler = MailHandler::new(sender, max_message_size);
        Ok(Self {
            inner,
            session_builder,
            mail_handler,
        })
    }

    pub fn local_addr(&self) -> Result<SocketAddr> {
        Ok(self.inner.local_addr()?)
    }

    pub async fn serve(&self) {
        loop {
            match self.inner.accept().await {
                Ok((stream, _)) => {
                    let session_builder = Arc::clone(&self.session_builder);
                    let mail_handler = self.mail_handler.clone();

                    tokio::spawn(async move {
                        match handle_connection(stream, session_builder, mail_handler).await {
                            Ok(()) => {}
                            Err(e) => {
                                tracing::error!(?e, "connection error");
                            }
                        }
                    });
                }
                Err(e) => {
                    tracing::error!(?e, "error while accepting connection");
                }
            }
        }
    }
}

async fn handle_connection(
    mut stream: TcpStream,
    session_builder: Arc<SessionBuilder>,
    handler: MailHandler,
) -> Result<()> {
    let peer_addr = stream.peer_addr()?;
    let mut session = session_builder.build(peer_addr.ip(), handler);

    // send initial greeting
    let greet = session.greeting();
    write_response(&mut stream, &greet).await?;

    // read a line
    let mut buf_reader = BufReader::new(stream);
    let mut line = String::new();

    loop {
        match buf_reader.read_line(&mut line).await {
            // TODO: currently reads utf-8, change to read raw bytes later
            Ok(0) => break,
            Ok(_) => {}
            Err(e) => {
                tracing::error!(?e, "error reading from connection");
                break;
            }
        }
        let response = session.process(line.as_bytes());
        line.clear(); // clear the line after for next iteration or else it would be a growing blob
        match response.action {
            Action::Reply => {
                write_response(&mut buf_reader, &response).await?;
            }
            Action::NoReply => {}
            Action::Close => {
                write_response(&mut buf_reader, &response).await?;
                break;
            }
            Action::UpgradeTls => {}
        }
    }
    Ok(())
}

async fn write_response<W>(writer: &mut W, res: &Response) -> Result<()>
where
    W: AsyncWrite + Unpin,
{
    let res = res.buffer()?;
    writer.write_all(&res).await?;
    writer.flush().await?;
    Ok(())
}
