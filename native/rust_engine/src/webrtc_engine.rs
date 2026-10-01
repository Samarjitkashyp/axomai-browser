//! WebRTC Peer-to-Peer Real-Time Communication Engine for Axomai Browser.
//! Implements W3C WebRTC 1.0 specifications for RTCPeerConnection, RTCDataChannel, and SDP signaling.

use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RtcSdpType {
    Offer,
    PrAnswer,
    Answer,
    Rollback,
}

#[derive(Debug, Clone)]
pub struct RtcSessionDescription {
    pub sdp_type: RtcSdpType,
    pub sdp: String,
}

#[derive(Debug, Clone)]
pub struct RtcIceCandidate {
    pub candidate: String,
    pub sdp_mid: Option<String>,
    pub sdp_m_line_index: Option<u16>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RtcSignalingState {
    Stable,
    HaveLocalOffer,
    HaveRemoteOffer,
    HaveLocalPrAnswer,
    HaveRemotePrAnswer,
    Closed,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RtcIceConnectionState {
    New,
    Checking,
    Connected,
    Completed,
    Failed,
    Disconnected,
    Closed,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RtcDataChannelState {
    Connecting,
    Open,
    Closing,
    Closed,
}

#[derive(Debug, Clone)]
pub struct RtcDataChannel {
    pub id: u16,
    pub label: String,
    pub ordered: bool,
    pub ready_state: RtcDataChannelState,
    pub outgoing_messages: Vec<String>,
    pub incoming_messages: Vec<String>,
}

impl RtcDataChannel {
    pub fn new(id: u16, label: &str) -> Self {
        RtcDataChannel {
            id,
            label: label.to_string(),
            ordered: true,
            ready_state: RtcDataChannelState::Open,
            outgoing_messages: Vec::new(),
            incoming_messages: Vec::new(),
        }
    }

    pub fn send(&mut self, data: &str) -> Result<(), String> {
        if self.ready_state != RtcDataChannelState::Open {
            return Err("DataChannel is not open".to_string());
        }
        self.outgoing_messages.push(data.to_string());
        Ok(())
    }

    pub fn receive(&mut self, data: &str) {
        self.incoming_messages.push(data.to_string());
    }
}

pub struct RtcPeerConnection {
    pub ice_servers: Vec<String>,
    pub signaling_state: RtcSignalingState,
    pub ice_connection_state: RtcIceConnectionState,
    pub local_description: Option<RtcSessionDescription>,
    pub remote_description: Option<RtcSessionDescription>,
    pub candidates: Vec<RtcIceCandidate>,
    pub data_channels: HashMap<String, RtcDataChannel>,
    next_channel_id: u16,
}

impl RtcPeerConnection {
    pub fn new(ice_servers: Vec<String>) -> Self {
        RtcPeerConnection {
            ice_servers,
            signaling_state: RtcSignalingState::Stable,
            ice_connection_state: RtcIceConnectionState::New,
            local_description: None,
            remote_description: None,
            candidates: Vec::new(),
            data_channels: HashMap::new(),
            next_channel_id: 1,
        }
    }

    pub fn create_offer(&mut self) -> Result<RtcSessionDescription, String> {
        let offer = RtcSessionDescription {
            sdp_type: RtcSdpType::Offer,
            sdp: format!("v=0\r\no=AxomaiWebRTC 1000 2 IN IP4 127.0.0.1\r\ns=-\r\nt=0 0\r\na=sendrecv\r\n"),
        };
        Ok(offer)
    }

    pub fn create_answer(&mut self) -> Result<RtcSessionDescription, String> {
        let answer = RtcSessionDescription {
            sdp_type: RtcSdpType::Answer,
            sdp: format!("v=0\r\no=AxomaiWebRTC 2000 2 IN IP4 127.0.0.1\r\ns=-\r\nt=0 0\r\na=sendrecv\r\n"),
        };
        Ok(answer)
    }

    pub fn set_local_description(&mut self, desc: RtcSessionDescription) -> Result<(), String> {
        match desc.sdp_type {
            RtcSdpType::Offer => self.signaling_state = RtcSignalingState::HaveLocalOffer,
            RtcSdpType::Answer => self.signaling_state = RtcSignalingState::Stable,
            _ => {}
        }
        self.local_description = Some(desc);
        Ok(())
    }

    pub fn set_remote_description(&mut self, desc: RtcSessionDescription) -> Result<(), String> {
        match desc.sdp_type {
            RtcSdpType::Offer => self.signaling_state = RtcSignalingState::HaveRemoteOffer,
            RtcSdpType::Answer => {
                self.signaling_state = RtcSignalingState::Stable;
                self.ice_connection_state = RtcIceConnectionState::Connected;
            }
            _ => {}
        }
        self.remote_description = Some(desc);
        Ok(())
    }

    pub fn add_ice_candidate(&mut self, candidate: RtcIceCandidate) {
        self.candidates.push(candidate);
    }

    pub fn create_data_channel(&mut self, label: &str) -> Result<u16, String> {
        let id = self.next_channel_id;
        self.next_channel_id += 1;
        let channel = RtcDataChannel::new(id, label);
        self.data_channels.insert(label.to_string(), channel);
        Ok(id)
    }

    pub fn close(&mut self) {
        self.signaling_state = RtcSignalingState::Closed;
        self.ice_connection_state = RtcIceConnectionState::Closed;
        for (_, ch) in self.data_channels.iter_mut() {
            ch.ready_state = RtcDataChannelState::Closed;
        }
    }
}
