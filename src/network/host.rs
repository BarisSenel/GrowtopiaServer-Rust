use rusty_enet as enet;
use std::net::{Ipv4Addr, SocketAddr, SocketAddrV4, UdpSocket};
use std::str::FromStr;
use crate::network::peer::NativePeer;

#[derive(Debug)]
pub enum HostEvent {
    Connect {
        peer_id: u32,
    },
    Disconnect {
        peer_id: u32,
        data: u32,
    },
    Receive {
        peer_id: u32,
        channel_id: u8,
        data: Vec<u8>,
    },
}

pub struct Host {
    host: enet::Host<UdpSocket>,
}

impl Host {
    pub fn new(
        ip_address: &str,
        port: u16,
        peer_limit: u32,
        channel_limit: u8,
        using_new_packet: bool,
        using_new_packet_server: bool,
        incoming_bandwidth_limit: Option<u32>,
        outgoing_bandwidth_limit: Option<u32>,
        enable_compressor: bool,
        enable_checksum: bool,
    ) -> Result<Self, String> {
        let socket = if using_new_packet || using_new_packet_server {
            UdpSocket::bind(SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::UNSPECIFIED, port)))
                .map_err(|e| e.to_string())?
        } else {
            let addr = format!("{}:{}", ip_address, port);
            UdpSocket::bind(SocketAddr::from_str(&addr).map_err(|e| e.to_string())?)
                .map_err(|e| e.to_string())?
        };

        let host = enet::Host::new(
            socket,
            enet::HostSettings {
                peer_limit: peer_limit as usize,
                channel_limit: channel_limit as usize,
                incoming_bandwidth_limit,
                outgoing_bandwidth_limit,
                using_new_packet,
                using_new_packet_server,
                compressor: if enable_compressor {
                    Some(Box::new(enet::RangeCoder::new()))
                } else {
                    None
                },
                checksum: if enable_checksum {
                    Some(Box::new(enet::crc32))
                } else {
                    None
                },
                ..Default::default()
            },
        )
        .map_err(|e| format!("Failed to create host: {}", e))?;

        Ok(Host { host })
    }

    pub fn service(&mut self) -> Result<Option<HostEvent>, String> {
        match self.host.service() {
            Ok(Some(event)) => match event {

                enet::Event::Connect { peer, .. } => Ok(Some(HostEvent::Connect {
                    peer_id: peer.id().0 as u32,
                })),
                enet::Event::Receive {
                    peer,
                    packet,
                    channel_id,
                    ..
                } => Ok(Some(HostEvent::Receive {
                    peer_id: peer.id().0 as u32,
                    channel_id,
                    data: packet.data().to_vec(),
                })),
                enet::Event::Disconnect { peer, data } => Ok(Some(HostEvent::Disconnect {
                    peer_id: peer.id().0 as u32,
                    data,
                })),
            },
            Ok(None) => Ok(None),
            Err(e) => Err(format!("Service error: {}", e)),
        }
    }



    pub fn ip_address(&self) -> String {
        self.host.socket().local_addr().map(|a| a.ip().to_string()).unwrap_or_default()
    }

    pub fn port(&self) -> u16 {
        self.host.socket().local_addr().map(|a| a.port()).unwrap_or(0)
    }

    pub fn get_peer(&mut self, net_id: u32) -> Result<NativePeer, String> {
        if let Some(peer) = self.host.peers().nth(net_id as usize) {
            let addr = peer.address().unwrap_or(SocketAddr::from(([0,0,0,0], 0)));
            Ok(NativePeer {
                net_id,
                state: peer.state() as u8,
                ip: addr.ip().to_string(),
                port: addr.port(),
                rtt: peer.round_trip_time().as_millis() as u32,
                round_trip_time: peer.round_trip_time().as_millis() as u32,
                round_trip_time_variance: peer.round_trip_time_variance().as_millis() as u32,
                mtu: peer.mtu(),
                channel_count: peer.channel_count() as u32,
                incoming_bandwidth: peer.incoming_bandwidth(),
                outgoing_bandwidth: peer.outgoing_bandwidth(),
                incoming_bandwidth_throttle_epoch: 0,
                outgoing_bandwidth_throttle_epoch: 0,
                ping_interval: peer.ping_interval().as_millis() as u32,
                timeout_limit: 0,
                timeout_minimum: 0,
                timeout_maximum: 0,
                last_round_trip_time_variance: 0,
                last_round_trip_time: 0,
                lowest_round_trip_time: 0,
                packet_throttle_interval: 0,
                packet_throttle_acceleration: 0,
                packet_throttle_deceleration: 0,
                packet_throttle: 0,
                packets_sent: peer.packets_sent() as u32,
                packets_lost: peer.packets_lost() as u32,
                packet_loss: peer.packet_loss() as u32,
                packet_loss_variance: peer.packet_loss_variance() as u32,
                incoming_data_total: peer.incoming_data_total() as u32,
                outgoing_data_total: peer.outgoing_data_total() as u32,
            })
        } else {
            Err("Peer not found".to_string())
        }
    }

    pub fn send(&mut self, net_id: u32, data: &[u8], channel_id: u8) -> Result<(), String> {

        if let Some(peer) = self.host.peers_mut().nth(net_id as usize) {
            let packet = enet::Packet::reliable(data);
            peer.send(channel_id, &packet).map_err(|e| e.to_string())
        } else {
            Err("Peer not found".to_string())
        }
    }

    pub fn disconnect_peer(&mut self, net_id: u32, data: u32) -> Result<(), String> {
        if let Some(peer) = self.host.peers_mut().nth(net_id as usize) {
            peer.disconnect(data);
            Ok(())
        } else {
            Err("Peer not found".to_string())
        }
    }

    pub fn disconnect_now_peer(&mut self, net_id: u32, data: u32) -> Result<(), String> {
        if let Some(peer) = self.host.peers_mut().nth(net_id as usize) {
            peer.disconnect_now(data);
            Ok(())
        } else {
            Err("Peer not found".to_string())
        }
    }

    pub fn disconnect_later_peer(&mut self, net_id: u32, data: u32) -> Result<(), String> {
        if let Some(peer) = self.host.peers_mut().nth(net_id as usize) {
            peer.disconnect_later(data);
            Ok(())
        } else {
            Err("Peer not found".to_string())
        }
    }

    pub fn ping_peer(&mut self, net_id: u32) -> Result<(), String> {
        if let Some(peer) = self.host.peers_mut().nth(net_id as usize) {
            peer.ping();
            Ok(())
        } else {
            Err("Peer not found".to_string())
        }
    }

    pub fn reset_peer(&mut self, net_id: u32) -> Result<(), String> {
        if let Some(peer) = self.host.peers_mut().nth(net_id as usize) {
            peer.reset();
            Ok(())
        } else {
            Err("Peer not found".to_string())
        }
    }

    pub fn set_timeout_peer(&mut self, net_id: u32, limit: u32, min: u32, max: u32) -> Result<(), String> {
        if let Some(peer) = self.host.peers_mut().nth(net_id as usize) {
            peer.set_timeout(limit, min, max);
            Ok(())
        } else {
            Err("Peer not found".to_string())
        }
    }

    pub fn set_ping_interval_peer(&mut self, net_id: u32, val: u32) -> Result<(), String> {
        if let Some(peer) = self.host.peers_mut().nth(net_id as usize) {
            peer.set_ping_interval(val);
            Ok(())
        } else {
            Err("Peer not found".to_string())
        }
    }

    pub fn set_throttle_peer(&mut self, net_id: u32, interval: u32, acc: u32, dec: u32) -> Result<(), String> {
        if let Some(peer) = self.host.peers_mut().nth(net_id as usize) {
            peer.set_throttle(interval, acc, dec);
            Ok(())
        } else {
            Err("Peer not found".to_string())
        }
    }

    pub fn set_mtu_peer(&mut self, net_id: u32, mtu: u16) -> Result<(), String> {
         if let Some(peer) = self.host.peers_mut().nth(net_id as usize) {
             peer.set_mtu(mtu).map_err(|e| e.to_string())
        } else {
            Err("Peer not found".to_string())
        }
    }
}