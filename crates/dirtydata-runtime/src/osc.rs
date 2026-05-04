use crate::nodes::OscMessage;
use crate::{EngineCommand, ParameterUpdate, StableId};
use crossbeam_channel::{Receiver, Sender};
use rosc::{OscMessage as RoscMessage, OscPacket};
use std::net::UdpSocket;

pub struct OscHandler {
    command_tx: Sender<EngineCommand>,
}

impl OscHandler {
    pub fn new(command_tx: Sender<EngineCommand>) -> Self {
        Self { command_tx }
    }

    pub fn spawn_input_thread(&self, addr: &str) {
        let command_tx = self.command_tx.clone();
        let addr = addr.to_string();
        std::thread::spawn(move || {
            let socket = match UdpSocket::bind(&addr) {
                Ok(s) => s,
                Err(e) => {
                    tracing::error!("Failed to bind OSC input socket to {}: {}", addr, e);
                    return;
                }
            };
            let mut buf = [0u8; 4096];
            loop {
                match socket.recv_from(&mut buf) {
                    Ok((size, _)) => {
                        if let Ok((_, OscPacket::Message(msg))) =
                            rosc::decoder::decode_udp(&buf[..size])
                        {
                            Self::handle_message(&command_tx, msg);
                        }
                    }
                    Err(e) => {
                        tracing::error!("OSC input recv error: {}", e);
                        break;
                    }
                }
            }
        });
    }

    fn handle_message(command_tx: &Sender<EngineCommand>, msg: RoscMessage) {
        let parts: Vec<&str> = msg
            .addr
            .split('/')
            .filter(|s: &&str| !s.is_empty())
            .collect();
        if parts.len() == 3 && parts[0] == "node" {
            if let (Ok(node_id), Some(val)) = (parts[1].parse::<StableId>(), msg.args.first()) {
                let float_val = match *val {
                    rosc::OscType::Float(f) => f,
                    rosc::OscType::Double(d) => d as f32,
                    rosc::OscType::Int(i) => i as f32,
                    _ => 0.0,
                };
                let _ = command_tx.send(EngineCommand::UpdateParameter(ParameterUpdate {
                    node_id,
                    param: parts[2].to_string(),
                    value: float_val,
                    provenance: vec!["osc".to_string()],
                }));
            }
        }
    }

    pub fn spawn_output_thread(rx: Receiver<OscMessage>, target_addr: String) {
        std::thread::spawn(move || {
            let socket = match UdpSocket::bind("0.0.0.0:0") {
                Ok(s) => s,
                Err(e) => {
                    tracing::error!("Failed to bind OSC output socket: {}", e);
                    return;
                }
            };
            while let Ok(msg) = rx.recv() {
                let packet = OscPacket::Message(RoscMessage {
                    addr: msg.addr,
                    args: msg.args,
                });
                if let Ok(bytes) = rosc::encoder::encode(&packet) {
                    let b: Vec<u8> = bytes;
                    let _ = socket.send_to(&b, &target_addr);
                }
            }
        });
    }
}
