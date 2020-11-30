use crate::asyncio::{AsyncVec, SizeCalc};
use crate::datatypes::*;
use crate::packets::{ClientBound, ServerBound};
use flate2::read::ZlibDecoder;
use flate2::write::ZlibEncoder;
use tokio::io::{self, AsyncReadExt};
use tokio::net::TcpStream;
use tokio::prelude::*;

pub struct Client {
    pub stream: TcpStream,
    pub status: i64, // 0 - handshake, 1 - status, 2 - login, 3 - play
    pub compression: i64,
}

impl Client {
    pub async fn send(&mut self, packet: ServerBound) -> io::Result<()> {
        // change the status as needed
        if let ServerBound::Handshake(_, _, _, next_state) = &packet {
            self.status = next_state.0;
        }

        let mut size = SizeCalc(0);
        packet.clone().gen_to(&mut size).await?;

        if self.compression > 0 {
            // packet is sent in the compressed format
            if size.0 as i64 > self.compression {
                // need to compress the packet
                let mut encoder =
                    ZlibEncoder::new(AsyncVec(Vec::new()), flate2::Compression::fast());
                packet.gen_to(&mut encoder).await?;
                let data = encoder.finish()?.0;
                let uncompressed_data_size = size.0 as i64;
                VarInt(uncompressed_data_size).serialize(&mut size).await?;

                // the whole packet size
                VarInt(size.0 as i64).serialize(&mut self.stream).await?;
                // the uncompressed data size
                VarInt(uncompressed_data_size)
                    .serialize(&mut self.stream)
                    .await?;
                // the actual compressed data
                self.stream.write_all(&data[..]).await?;
            } else {
                // no need to compress the packet
                VarInt(size.0 as i64).serialize(&mut size).await?;

                // the whole packet size
                VarInt(size.0 as i64).serialize(&mut self.stream).await?;
                // uncompressed data size, which is 0, because its uncompressed
                VarInt(0).serialize(&mut self.stream).await?;
                // the actual data
                packet.gen_to(&mut self.stream).await?;
            }
        } else {
            // not compressed format
            VarInt(size.0 as i64).serialize(&mut self.stream).await?;
            packet.gen_to(&mut self.stream).await?;
        }

        Ok(())
    }
    pub async fn receive(&mut self) -> io::Result<ClientBound> {
        let packet_length = VarInt::deserialize(&mut self.stream).await?.0;

        let packet = if self.compression > 0 {
            let uncompressed_data_size = VarInt::deserialize(&mut self.stream).await?;
            // the packet is in the compressed format
            // the data is not necessarilly compressed yet
            if uncompressed_data_size.0 > 0 {
                // the data is compressed
                // calculate the number of bytes to read
                // gotta count how many bytes was the previous varint
                let bytes = uncompressed_data_size.size();

                let to_read = packet_length - bytes as i64;
                let mut bytes = vec![0u8; to_read as usize];
                self.stream.read_exact(&mut bytes).await?;

                let mut decoder = ZlibDecoder::new(std::io::Cursor::new(bytes));

                Ok(ClientBound::read_from(
                    &mut decoder,
                    packet_length - uncompressed_data_size.size() as i64,
                    self.status,
                )
                .await?)
            } else {
                // the data is not compressed
                Ok(ClientBound::read_from(
                    &mut self.stream,
                    packet_length - uncompressed_data_size.size() as i64,
                    self.status,
                )
                .await?)
            }
        } else {
            // the packet is in the normal format
            Ok(ClientBound::read_from(&mut self.stream, packet_length, self.status).await?)
        };

        // change the status as needed
        if let Ok(ClientBound::LoginSuccess(..)) = &packet {
            self.status = 3;
        }
        // and compression
        if let Ok(ClientBound::SetCompression(compression)) = &packet {
            self.compression = compression.0;
        }

        packet
    }
}
