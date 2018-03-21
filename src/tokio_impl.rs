use super::*;
use tokio::prelude::*;
use tokio::io::{ AsyncRead, AsyncWrite };
use tokio::prelude::Poll;


impl<S: AsyncRead + AsyncWrite> Future for ConnectAsync<S> {
    type Item = TlsStream<S, ClientSession>;
    type Error = io::Error;

    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        self.0.poll()
    }
}

impl<S: AsyncRead + AsyncWrite> Future for AcceptAsync<S> {
    type Item = TlsStream<S, ServerSession>;
    type Error = io::Error;

    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        self.0.poll()
    }
}

impl<S, C> Future for MidHandshake<S, C>
    where S: io::Read + io::Write, C: Session
{
    type Item = TlsStream<S, C>;
    type Error = io::Error;

    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        loop {
            let stream = self.inner.as_mut().unwrap();
            if !stream.session.is_handshaking() { break };

            match stream.do_io() {
                Ok(()) => match (stream.eof, stream.session.is_handshaking()) {
                    (true, true) => return Err(io::Error::from(io::ErrorKind::UnexpectedEof)),
                    (false, true) => continue,
                    (..) => break
                },
                Err(e) => match (e.kind(), stream.session.is_handshaking()) {
                    (io::ErrorKind::WouldBlock, true) => return Ok(Async::NotReady),
                    (io::ErrorKind::WouldBlock, false) => break,
                    (..) => return Err(e)
                }
            }
        }

        Ok(Async::Ready(self.inner.take().unwrap()))
    }
}

impl<S, C> AsyncRead for TlsStream<S, C>
    where
        S: AsyncRead + AsyncWrite,
        C: Session
{}

impl<S, C> AsyncWrite for TlsStream<S, C>
    where
        S: AsyncRead + AsyncWrite,
        C: Session
{
    fn shutdown(&mut self) -> Poll<(), io::Error> {
        if !self.is_shutdown {
            self.session.send_close_notify();
            self.is_shutdown = true;
        }
        while self.session.wants_write() {
            self.session.write_tls(&mut self.io)?;
        }
        self.io.flush()?;
        self.io.shutdown()
    }
}
