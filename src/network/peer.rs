
use crate::network::host::Host;


#[derive(Debug, Clone)]
pub struct NativePeer {

    pub net_id: u32,
    pub state: u8,
    pub ip: String,
    pub port: u16,
    pub rtt: u32,
    pub round_trip_time: u32,
    pub round_trip_time_variance: u32,
    pub mtu: u16,
    pub channel_count: u32,
    pub incoming_bandwidth: u32,
    pub outgoing_bandwidth: u32,
    pub incoming_bandwidth_throttle_epoch: u32,
    pub outgoing_bandwidth_throttle_epoch: u32,
    pub ping_interval: u32,
    pub timeout_limit: u32,
    pub timeout_minimum: u32,
    pub timeout_maximum: u32,
    pub last_round_trip_time_variance: u32,
    pub last_round_trip_time: u32,
    pub lowest_round_trip_time: u32,
    pub packet_throttle_interval: u32,
    pub packet_throttle_acceleration: u32,
    pub packet_throttle_deceleration: u32,
    pub packet_throttle: u32,
    pub packets_sent: u32,
    pub packets_lost: u32,
    pub packet_loss: u32,
    pub packet_loss_variance: u32,
    pub incoming_data_total: u32,
    pub outgoing_data_total: u32,
}

impl NativePeer {

    pub fn connected(&self) -> bool {
        self.state == 1
    }



    pub fn send(&self, host: &mut Host, channel_id: u8, data: &[u8]) -> Result<(), String> {
        host.send(self.net_id, data, channel_id)
    }

    pub fn disconnect(&self, host: &mut Host, data: u32) -> Result<(), String> {
        host.disconnect_peer(self.net_id, data)
    }


    pub fn ping(&self, host: &mut Host) -> Result<(), String> {
        host.ping_peer(self.net_id)
    }


}