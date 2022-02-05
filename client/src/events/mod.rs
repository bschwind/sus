use sus_common::network::{ClientToServer, DeliveryType};

#[derive(Debug)]
pub struct OutgoingPacket {
    pub packet: ClientToServer,
    pub delivery_type: DeliveryType,
    pub stream_id: Option<u8>,
}

impl OutgoingPacket {
    pub fn new(packet: ClientToServer, delivery_type: DeliveryType, stream_id: Option<u8>) -> Self {
        Self { packet, delivery_type, stream_id }
    }
}
