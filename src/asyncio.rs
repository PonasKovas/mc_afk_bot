use async_trait::async_trait;
use core::future::poll_fn;
use core::task::Poll;
use flate2::read::ZlibDecoder;
use flate2::write::ZlibEncoder;
use std::io::Write;
use tokio::io::{self, AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio_io::{AsyncRead, AsyncWrite};

// General AsyncWrite and AsyncRead traits that are implemented both for tokio's TcpStreams
// and flate2's zlib decoders/encoders
#[async_trait]
pub trait MyAsyncRead {
    async fn read(&mut self, output: &mut [u8]) -> io::Result<()>;

    async fn read_byte(self: &mut Self) -> io::Result<u8> {
        let mut byte = [0u8; 1];
        self.read(&mut byte[..]).await?;

        Ok(byte[0])
    }
}

#[async_trait]
pub trait MyAsyncWrite {
    async fn write(&mut self, input: &[u8]) -> io::Result<()>;

    async fn write_byte(self: &mut Self, byte: u8) -> io::Result<()> {
        let byte = [byte];
        self.write(&byte[..]).await?;

        Ok(())
    }
}

// acts as an input stream but all it does is count bytes
#[derive(Debug)]
pub struct SizeCalc(pub u64);

// A newtype Vec that implements tokio_io::AsyncWrite
#[derive(Debug, Clone)]
pub struct AsyncVec(pub Vec<u8>);

impl Write for AsyncVec {
    fn write(&mut self, s: &[u8]) -> io::Result<usize> {
        Write::write(&mut self.0, s)
    }
    fn flush(&mut self) -> io::Result<()> {
        Write::flush(&mut self.0)
    }
}

impl AsyncWrite for AsyncVec {
    fn shutdown(&mut self) -> futures::Poll<(), std::io::Error> {
        // no idea why this method even exists so just
        Ok(futures::Async::Ready(()))
    }
}

#[async_trait]
impl MyAsyncWrite for SizeCalc {
    async fn write(&mut self, input: &[u8]) -> io::Result<()> {
        self.0 += input.len() as u64;

        Ok(())
    }
}

#[async_trait]
impl MyAsyncRead for TcpStream {
    async fn read(&mut self, output: &mut [u8]) -> io::Result<()> {
        self.read_exact(output).await?;

        Ok(())
    }
}

#[async_trait]
impl<T: AsyncRead + Send> MyAsyncRead for ZlibDecoder<T> {
    async fn read(&mut self, output: &mut [u8]) -> io::Result<()> {
        poll_fn(|_| match self.poll_read(output) {
            Ok(futures::Async::Ready(t)) => Poll::Ready(Ok(t)),
            Ok(futures::Async::NotReady) => Poll::Pending,
            Err(e) => Poll::Ready(Err(e)),
        })
        .await?;

        Ok(())
    }
}

#[async_trait]
impl MyAsyncWrite for TcpStream {
    async fn write(&mut self, input: &[u8]) -> io::Result<()> {
        self.write_all(input).await?;

        Ok(())
    }
}

#[async_trait]
impl<T: AsyncWrite + Send> MyAsyncWrite for ZlibEncoder<T> {
    async fn write(&mut self, input: &[u8]) -> io::Result<()> {
        poll_fn(|_| match self.poll_write(input) {
            Ok(futures::Async::Ready(t)) => Poll::Ready(Ok(t)),
            Ok(futures::Async::NotReady) => Poll::Pending,
            Err(e) => Poll::Ready(Err(e)),
        })
        .await?;

        Ok(())
    }
}
