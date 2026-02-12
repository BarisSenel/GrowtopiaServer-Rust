
use bytes::{Buf, BufMut, BytesMut};

#[derive(Debug, Clone)]
pub struct GamePacket {
    pub packet_type: i32,
    pub net_id: i32,
    pub uid: i32,
    pub peer_state: i32,
    pub count: f32,
    pub id: i32,
    pub pos_x: f32,
    pub pos_y: f32,
    pub speed_x: f32,
    pub speed_y: f32,
    pub idk: i32,
    pub punch_x: i32,
    pub punch_y: i32,
}

impl GamePacket {
    pub fn new() -> Self {
        Self {
            packet_type: 0,
            net_id: -1,
            uid: -1,
            peer_state: 8,
            count: 0.0,
            id: 0,
            pos_x: 0.0,
            pos_y: 0.0,
            speed_x: 0.0,
            speed_y: 0.0,
            idk: 0,
            punch_x: 0,
            punch_y: 0,
        }
    }

    pub fn from_bytes(data: &[u8]) -> Option<(Self, Vec<u8>)> {
        if data.len() < 60 { return None; }

        let mut rdr = &data[..];

        let header = rdr.get_i32_le();
        if header != 4 && header != 10 { return None; }

        let packet = Self {
            packet_type: rdr.get_i32_le(),
            net_id: rdr.get_i32_le(),
            uid: rdr.get_i32_le(),
            peer_state: rdr.get_i32_le(),
            count: rdr.get_f32_le(),
            id: rdr.get_i32_le(),
            pos_x: rdr.get_f32_le(),
            pos_y: rdr.get_f32_le(),
            speed_x: rdr.get_f32_le(),
            speed_y: rdr.get_f32_le(),
            idk: rdr.get_i32_le(),
            punch_x: rdr.get_i32_le(),
            punch_y: rdr.get_i32_le(),
        };

        let extra_size = rdr.get_u32_le() as usize;
        let mut extra_data = Vec::new();
        if extra_size > 0 && rdr.len() >= extra_size {
            extra_data = rdr[..extra_size].to_vec();
        }

        Some((packet, extra_data))
    }

    pub fn to_bytes(&self, variant_data: &[u8], variant_count: u8) -> Vec<u8> {
        let mut buf = BytesMut::with_capacity(60 + variant_data.len());

        buf.put_i32_le(4);
        buf.put_i32_le(self.packet_type);
        buf.put_i32_le(self.net_id);
        buf.put_i32_le(self.uid);
        buf.put_i32_le(self.peer_state);
        buf.put_f32_le(self.count);
        buf.put_i32_le(self.id);
        buf.put_f32_le(self.pos_x);
        buf.put_f32_le(self.pos_y);
        buf.put_f32_le(self.speed_x);
        buf.put_f32_le(self.speed_y);
        buf.put_i32_le(self.idk);
        buf.put_i32_le(self.punch_x);
        buf.put_i32_le(self.punch_y);

        if variant_count > 0 {
            buf.put_u32_le(variant_data.len() as u32 + 1);
            buf.put_u8(variant_count);
            buf.put_slice(variant_data);
        } else {
            buf.put_u32_le(0);
        }

        buf.to_vec()
    }

    pub fn to_bytes_with_raw_data(&self, raw_data: &[u8]) -> Vec<u8> {
        let mut buf = BytesMut::with_capacity(60 + raw_data.len());

        buf.put_i32_le(4);
        buf.put_i32_le(self.packet_type);
        buf.put_i32_le(self.net_id);
        buf.put_i32_le(self.uid);
        buf.put_i32_le(self.peer_state);
        buf.put_f32_le(self.count);
        buf.put_i32_le(self.id);
        buf.put_f32_le(self.pos_x);
        buf.put_f32_le(self.pos_y);
        buf.put_f32_le(self.speed_x);
        buf.put_f32_le(self.speed_y);
        buf.put_i32_le(self.idk);
        buf.put_i32_le(self.punch_x);
        buf.put_i32_le(self.punch_y);

        buf.put_u32_le(raw_data.len() as u32);
        buf.put_slice(raw_data);

        buf.to_vec()
    }
}

pub struct VariantListBuilder {
    data: Vec<u8>,
    count: u8,
}

impl VariantListBuilder {
    pub fn new() -> Self {
        Self {
            data: Vec::new(),
            count: 0,
        }
    }

    pub fn add_string(mut self, val: &str) -> Self {
        self.data.push(self.count);
        self.data.push(0x02);
        self.data.put_i32_le(val.len() as i32);
        self.data.put_slice(val.as_bytes());
        self.count += 1;
        self
    }

    pub fn add_int(mut self, val: i32) -> Self {
        self.data.push(self.count);
        self.data.push(0x09);
        self.data.put_i32_le(val);
        self.count += 1;
        self
    }

    pub fn add_uint(mut self, val: u32) -> Self {
        self.data.push(self.count);
        self.data.push(0x05);
        self.data.put_u32_le(val);
        self.count += 1;
        self
    }


    pub fn add_float(mut self, val: f32) -> Self {
        self.data.push(self.count);
        self.data.push(0x01);
        self.data.put_f32_le(val);
        self.count += 1;
        self
    }

    pub fn add_vec2(mut self, x: f32, y: f32) -> Self {
        self.data.push(self.count);
        self.data.push(0x03);
        self.data.put_f32_le(x);
        self.data.put_f32_le(y);
        self.count += 1;
        self
    }

    pub fn add_vec3(mut self, x: f32, y: f32, z: f32) -> Self {
        self.data.push(self.count);
        self.data.push(0x04);
        self.data.put_f32_le(x);
        self.data.put_f32_le(y);
        self.data.put_f32_le(z);
        self.count += 1;
        self
    }

    pub fn build(self) -> (Vec<u8>, u8) {
        (self.data, self.count)
    }
}