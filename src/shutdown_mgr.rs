use tokio::sync::{broadcast, mpsc};

use crate::protocol::UFT_SERVER_MAX_SYM;

#[derive(Clone, Debug)]
pub enum ChannelState {
    Complete(std::net::SocketAddr, String), // Client, File sent
    Error(String), // Error message
    Shutdown
}


#[derive(Debug)]
pub struct ChannelManager {
    broadcast_rx: broadcast::Receiver<ChannelState>,
    broadcast_sx: broadcast::Sender::<ChannelState>,
    state_sx: Option<mpsc::Sender<ChannelState>>,
}

impl ChannelManager {
    pub fn new(broadcast_sx: broadcast::Sender<ChannelState>, state_sx: Option<mpsc::Sender<ChannelState>>) -> ChannelManager{
        let broadcast_rx = broadcast_sx.subscribe();
        ChannelManager {
            broadcast_rx,
            broadcast_sx,
            state_sx
        }
    }

    pub async fn recv(&mut self) {
        let _ = self.broadcast_rx.recv().await;
    }

    pub fn send(&self, state: ChannelState){
        if self.state_sx.is_none() {
            return;
        }
        let _ = self.state_sx.as_ref().unwrap().send(state);
    }

    pub fn shutdown(&mut self) {
        self.broadcast_sx.send(ChannelState::Shutdown);
    }
}

impl Clone for ChannelManager {
    fn clone(&self) -> ChannelManager {
        ChannelManager {
            broadcast_rx: self.broadcast_sx.subscribe(),
            broadcast_sx: self.broadcast_sx.clone(),
            state_sx: self.state_sx.clone()
        }
    }
}