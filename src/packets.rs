use crate::asyncio::SizeCalc;
use crate::datatypes::*;
use crate::{MyAsyncRead, MyAsyncWrite};
use tokio::io;

// Sent from the client to the server
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub enum ServerBound {
    Handshake(VarInt, MString, u16, VarInt), // protocol, address, port, next state
    StatusRequest,
    LoginStart(MString), // username
    KeepAlive(i64),
    ChatMessage(MString), // the raw message, up to 256 characters
    ClientStatus(VarInt), // 0 - respawn, 1 - request statistics
    InteractEntity(VarInt, VarInt, bool), // entity id, [0 - interact, 1 - attack, 2 - interact at (not supported)], whether sneaking
    PlayerPositionAndRotation(f64, f64, f64, f32, f32, bool), // x, y, z, yaw, pitch, whether on ground
    Animation(VarInt),                                        // 0 - main hand, 1 - off hand
    TeleportConfirm(VarInt),                                  // teleport id
    EntityAction(VarInt, VarInt, VarInt), // player's entity id, action (see https://wiki.vg/index.php?title=Protocol&oldid=16091#Entity_Action), jump boost (only for jumping with horse)
    HeldItemChange(i16),                  // slot id 0-8
    UseItem(VarInt),                      // 0 - main hand, 1 - off hand
    PlayerDigging(VarInt, i64, i8),       // action [0-6], position, face
}

// Sent from the server to the client
#[derive(Debug, Clone)]
pub enum ClientBound {
    LoginDisconnect(MString),
    StatusResponse(MString),
    SetCompression(VarInt),      // treshold
    LoginSuccess(u128, MString), // UUID and Username
    KeepAlive(i64),              // some random number that the client must respond with
    PlayDisconnect(MString),
    UpdateHealth(f32, VarInt, f32), // health, food, saturation
    PlayerPositionAndLook(f64, f64, f64, f32, f32, u8, VarInt), // x, y, z, yaw, pitch, flags, tp id
    SpawnLivingEntity(
        VarInt,
        u128,
        VarInt,
        f64,
        f64,
        f64,
        u8,
        u8,
        u8,
        i16,
        i16,
        i16,
    ), // entity id, uuid, type, x, y, z, yaw, pitch, head pitch, velocity: x, y, z
    EntityTeleport(VarInt, f64, f64, f64, u8, u8, bool), // entity id, x, y, z, yaw, pitch, whether on ground
    EntityPosition(VarInt, i16, i16, i16, bool), // entity id, delta x, y ,z, whether on ground
    DestroyEntities(Vec<VarInt>),                // Array of entity IDs to destroy
    JoinGame(i32), // this has lots of other data, but we're reading only the entity id
    SetSlot(i8, i16, Slot), // window id, slot id, slot data
    Statistics(Vec<(VarInt, VarInt, VarInt)>), // Category, id, value
    Unknown(VarInt), // the packet id of the unknown packet
}

impl ServerBound {
    pub async fn gen_to<O: MyAsyncWrite + Send + 'static>(self, output: &mut O) -> io::Result<()> {
        match self {
            Self::Handshake(protocol, address, port, next_state) => {
                VarInt(0x00).serialize(output).await?;

                protocol.serialize(output).await?;
                address.serialize(output).await?;
                port.serialize(output).await?;
                next_state.serialize(output).await?;
            }
            Self::StatusRequest => {
                VarInt(0x00).serialize(output).await?;
            }
            Self::LoginStart(username) => {
                VarInt(0x00).serialize(output).await?;

                username.serialize(output).await?;
            }
            Self::KeepAlive(id) => {
                VarInt(0x10).serialize(output).await?;

                id.serialize(output).await?;
            }
            Self::ChatMessage(message) => {
                VarInt(0x03).serialize(output).await?;

                message.serialize(output).await?;
            }
            Self::ClientStatus(what) => {
                VarInt(0x04).serialize(output).await?;

                what.serialize(output).await?;
            }
            Self::InteractEntity(entity_id, action, sneaking) => {
                VarInt(0x0E).serialize(output).await?;

                entity_id.serialize(output).await?;
                action.serialize(output).await?;
                sneaking.serialize(output).await?;
            }
            Self::PlayerPositionAndRotation(x, y, z, yaw, pitch, on_ground) => {
                VarInt(0x13).serialize(output).await?;

                x.serialize(output).await?;
                y.serialize(output).await?;
                z.serialize(output).await?;
                yaw.serialize(output).await?;
                pitch.serialize(output).await?;
                on_ground.serialize(output).await?;
            }
            Self::Animation(hand) => {
                VarInt(0x2c).serialize(output).await?;

                hand.serialize(output).await?;
            }
            Self::TeleportConfirm(id) => {
                VarInt(0x00).serialize(output).await?;

                id.serialize(output).await?;
            }
            Self::EntityAction(id, action, jump_boost) => {
                VarInt(0x1c).serialize(output).await?;

                id.serialize(output).await?;
                action.serialize(output).await?;
                jump_boost.serialize(output).await?;
            }
            Self::HeldItemChange(slot_id) => {
                VarInt(0x25).serialize(output).await?;

                slot_id.serialize(output).await?;
            }
            Self::UseItem(hand) => {
                VarInt(0x2F).serialize(output).await?;

                hand.serialize(output).await?;
            }
            Self::PlayerDigging(action, position, face) => {
                VarInt(0x1B).serialize(output).await?;

                action.serialize(output).await?;
                position.serialize(output).await?;
                face.serialize(output).await?;
            }
        }

        Ok(())
    }
}

impl ClientBound {
    pub async fn read_from<S: MyAsyncRead + Send + 'static>(
        input: &mut S,
        length: i64,
        status: i64,
    ) -> io::Result<Self> {
        let packet_id = VarInt::deserialize(input).await?.0;

        let result = match status {
            0 => {
                // Handshake
                // there are no packets to receive during this state
                Ok(Self::Unknown(VarInt(packet_id)))
            }
            1 => {
                // status
                match packet_id {
                    0x00 => Ok(Self::StatusResponse(MString::deserialize(input).await?)),
                    _ => Ok(Self::Unknown(VarInt(packet_id))),
                }
            }
            2 => {
                // login
                match packet_id {
                    0x00 => Ok(Self::LoginDisconnect(MString::deserialize(input).await?)),
                    0x02 => {
                        let uuid = u128::deserialize(input).await?;
                        let username = MString::deserialize(input).await?;

                        Ok(Self::LoginSuccess(uuid, username))
                    }
                    0x03 => Ok(Self::SetCompression(VarInt::deserialize(input).await?)),
                    _ => Ok(Self::Unknown(VarInt(packet_id))),
                }
            }
            3 => {
                // play
                match packet_id {
                    0x1F => Ok(Self::KeepAlive(i64::deserialize(input).await?)),
                    0x19 => Ok(Self::PlayDisconnect(MString::deserialize(input).await?)),
                    0x49 => Ok(Self::UpdateHealth(
                        f32::deserialize(input).await?,
                        VarInt::deserialize(input).await?,
                        f32::deserialize(input).await?,
                    )),
                    0x34 => Ok(Self::PlayerPositionAndLook(
                        f64::deserialize(input).await?,
                        f64::deserialize(input).await?,
                        f64::deserialize(input).await?,
                        f32::deserialize(input).await?,
                        f32::deserialize(input).await?,
                        u8::deserialize(input).await?,
                        VarInt::deserialize(input).await?,
                    )),
                    0x02 => Ok(Self::SpawnLivingEntity(
                        VarInt::deserialize(input).await?,
                        u128::deserialize(input).await?,
                        VarInt::deserialize(input).await?,
                        f64::deserialize(input).await?,
                        f64::deserialize(input).await?,
                        f64::deserialize(input).await?,
                        u8::deserialize(input).await?,
                        u8::deserialize(input).await?,
                        u8::deserialize(input).await?,
                        i16::deserialize(input).await?,
                        i16::deserialize(input).await?,
                        i16::deserialize(input).await?,
                    )),
                    0x56 => Ok(Self::EntityTeleport(
                        VarInt::deserialize(input).await?,
                        f64::deserialize(input).await?,
                        f64::deserialize(input).await?,
                        f64::deserialize(input).await?,
                        u8::deserialize(input).await?,
                        u8::deserialize(input).await?,
                        bool::deserialize(input).await?,
                    )),
                    0x27 => Ok(Self::EntityPosition(
                        VarInt::deserialize(input).await?,
                        i16::deserialize(input).await?,
                        i16::deserialize(input).await?,
                        i16::deserialize(input).await?,
                        bool::deserialize(input).await?,
                    )),
                    0x36 => Ok(Self::DestroyEntities(
                        Vec::<VarInt>::deserialize(input).await?,
                    )),
                    0x24 => {
                        let res = Ok(Self::JoinGame(i32::deserialize(input).await?));

                        // read to end
                        let size = length as usize - VarInt(packet_id).size() as usize - 4;
                        let mut garbage = vec![0u8; size];

                        input.read(&mut garbage).await?;

                        res
                    }
                    0x15 => {
                        let window_id = i8::deserialize(input).await?;
                        let slot_id = i16::deserialize(input).await?;
                        let slot = Slot::deserialize(input).await?;

                        let mut res_size = SizeCalc(0);
                        window_id.clone().serialize(&mut res_size).await?;
                        slot_id.clone().serialize(&mut res_size).await?;
                        slot.clone().serialize(&mut res_size).await?;

                        // read to end
                        let size = length as usize
                            - VarInt(packet_id).size() as usize
                            - res_size.0 as usize;
                        let mut garbage = vec![0u8; size];

                        input.read(&mut garbage).await?;

                        Ok(Self::SetSlot(window_id, slot_id, slot))
                    }
                    0x06 => {
                        let vec_size = VarInt::deserialize(input).await?;

                        let mut data = Vec::with_capacity(vec_size.0 as usize);
                        for _ in 0..vec_size.0 {
                            data.push((
                                VarInt::deserialize(input).await?,
                                VarInt::deserialize(input).await?,
                                VarInt::deserialize(input).await?,
                            ));
                        }

                        Ok(Self::Statistics(data))
                    }
                    _ => Ok(Self::Unknown(VarInt(packet_id))),
                }
            }
            _ => Ok(Self::Unknown(VarInt(packet_id))),
        };

        // if the packet is of unknown type, read the rest of the data, even though
        // it cant be understood, so that other packets can be parsed successfully
        if let Ok(Self::Unknown(_)) = &result {
            let size = length as usize - VarInt(packet_id).size() as usize;
            let mut garbage = vec![0u8; size];

            input.read(&mut garbage).await?;
        }

        result
    }
}
