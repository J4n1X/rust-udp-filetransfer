mod protocol;

use tokio::io::{BufReader, AsyncReadExt};
use tokio::net::{UdpSocket};
use tokio::sync::{broadcast, mpsc};
use tokio::fs::File;
use std::net::{SocketAddr,ToSocketAddrs};
use std::env;
use std::sync::Arc;
use crate::protocol::*;

#[derive(Debug, Clone)]
enum ClientServeReturnState {
    Complete(SocketAddr, String), // Client, File sent
    Error(String) // Error message
}

fn string_from_buffer(buf: &[u8]) -> String {
    String::from_utf8_lossy(&buf[..buf.iter()
        .position(|&r| r == 0u8).unwrap()])
        .into_owned()
}

struct ShutdownManager {
    shutdown: bool,
    receiver: broadcast::Receiver<()>,
    sender: broadcast::Sender<()>,
}

impl ShutdownManager {
    pub fn new(sender: broadcast::Sender<()>) -> ShutdownManager{
        let receiver = sender.subscribe();
        ShutdownManager {
            shutdown: false,
            receiver,
            sender
        }
    }

    pub fn is_shutdown(&self) -> bool {
        self.shutdown
    }
    
    pub async fn recv(&mut self) {
        if self.shutdown {
            return
        }

        let _ = self.receiver.recv().await;
        self.shutdown = true;
    }

    pub fn send(&mut self) {
        self.shutdown = true;
        self.sender.send(());
    }
}

impl Clone for ShutdownManager {
    fn clone(&self) -> ShutdownManager {
        ShutdownManager {
            shutdown: false,
            receiver: self.sender.subscribe(),
            sender: self.sender.clone()
        }
    }
}

// This doesn't really work at all, all it does is retrieve the external port.
async fn stun_request(socket: &UdpSocket, stun_server: &str) -> SocketAddr {
    use stunclient::StunClient;
    
    let stun_addr = stun_server
        .to_socket_addrs().unwrap()
        .filter(|x|x.is_ipv4())
        .next().unwrap();
    StunClient::new(stun_addr)
        .query_external_address_async(socket).await
        .expect("Request to STUN server failed!")
    
}


struct UdpServer {
    pub own_addr: SocketAddr,
    pub pub_addr: SocketAddr,
    pub socket: Arc<UdpSocket>,
    //clients: Arc<Mutex<Vec<ConnectedClient>>>,
    shutdown: ShutdownManager,
    max_sym_clients: usize,
}

impl UdpServer {
    // using https://crates.io/crates/stunclient
    pub async fn new(listen_ip: &str, stun_addr: &str, shutdownSender: broadcast::Sender<()>, sym_clients: usize) -> UdpServer {
        if sym_clients > 1 {
            panic!("Max simultaneous clients is less than 1");
        }
        let local_addr: SocketAddr = listen_ip.parse().unwrap();
        let udp_server_sock = UdpSocket::bind(&local_addr).await.unwrap();
        let pub_addr = stun_request(&udp_server_sock, stun_addr).await;
        UdpServer {
            own_addr: local_addr,
            pub_addr: pub_addr,
            socket: Arc::new(udp_server_sock),
            shutdown: ShutdownManager::new(shutdownSender),
            max_sym_clients: sym_clients
        }
    }

    pub async fn listen(&mut self) -> Result<(), ()> {
        let mut in_buffer = [0u8; UFT_BUFFER_SIZE];
        let (client_done_send, mut client_done_recv) = mpsc::channel::<ClientServeReturnState>(self.max_sym_clients);
        println!("Server is now listening");
        loop {
            tokio::select! {
                client_req = self.socket.recv_from(&mut in_buffer[..]) => {
                    let (_, client_addr) = client_req.expect("Receiving new Client failed");
                    match in_buffer[0] {
                        UFT_FILE_REQUEST => {
                            // create a new task to serve the file
                            self.serve_file(client_addr, string_from_buffer(&in_buffer[1..]), client_done_send.clone());
                        }
                        UFT_BLOCK_REQUEST => {
                            todo!("Open requested file and read specified block");
                        }
                        UFT_GENERAL_ERROR => {
                            // just report that we've failed (for now)
                            println!("Communication with {} has failed due to a general error", client_addr);
                        }
                        _ => {
                            panic!("Unknown mode: {}", in_buffer[0]);
                        }
                    }
                },
                serve_result = client_done_recv.recv() => {
                    match serve_result.unwrap() {
                        ClientServeReturnState::Complete(client, file) => {
                            println!("File {} has been sent to {}", file, client);
                        }
                        _ => {
                            println!("Unknown client result signal received");
                            self.shutdown.send();
                        }
                    }
                }
                _ = self.shutdown.recv() => {
                    println!("Terminating server...");
                    return Ok(());
                }
            }
        }
    }

    pub fn serve_file(&self, target_client: SocketAddr, target_file: String, done_sender: mpsc::Sender<ClientServeReturnState>) {
        use crate::protocol::UftServerStatus::*;

        println!("Serving file {} to client {}", target_file, target_client);
        let socket = self.socket.clone();
        let mut shutdown = self.shutdown.clone();
        tokio::spawn(async move {
            let mut file_reader = BufReader::new(File::open(&target_file).await.expect("Could not open file"));
            let mut sent_blocks: usize = 0;
            let mut buf_data = [0u8; UFT_BUFFER_SIZE];
            let read_bytes = file_reader.read(&mut buf_data[9..]).await.unwrap();
            let mut complete: bool = false;
            while !complete {
                // it is expected that an extra packet is sent if the file happens to fit perfectly into the buffers
                if read_bytes == 0{
                    return; // skip writing
                } else if read_bytes == UFT_BUFFER_SIZE {
                    buf_data[0] = FILE_DATA as u8;
                    buf_data[1..9].clone_from_slice(&sent_blocks.to_be_bytes());
                } else if read_bytes > 0 { 
                    buf_data[0] = FILE_COMPLETE as u8;
                    buf_data[1..9].clone_from_slice(&read_bytes.to_be_bytes());
                    complete = true;
                }
                tokio::select! {
                    send = socket.send_to(&buf_data, target_client) => {
                        send.expect("Failed to send data over UDP");
                        sent_blocks += 1;
                    },
                    _ = shutdown.recv() => {
                        println!("TODO: Send terminate signal");
                        return;
                    }
                };
            }
            println!("Wrote {} bytes to {}!", read_bytes, target_client);
            done_sender.send(ClientServeReturnState::Complete(target_client, target_file));
        });
    }
    
}

#[tokio::main]
async fn main() {
    match env::args().nth(1).unwrap().to_lowercase().as_str() {
        "client" => {
            let client_socket = UdpSocket::bind("0.0.0.0:0").await.unwrap();
            stun_request(&client_socket, "stun.l.google.com:19302").await;
            let mut buf = [0u8; UFT_BUFFER_SIZE];
            let file_path_bytes = "test.txt".as_bytes();
            let server: SocketAddr = env::args().nth(2).unwrap().parse().unwrap();
            buf[0] = UftClientStatus::FILE_REQUEST as u8;
            buf[1..(1 + file_path_bytes.len())].clone_from_slice(&file_path_bytes);
            client_socket.connect(server).await.unwrap();
            client_socket.send(&buf).await.unwrap();
            println!("Sent request!");
            return;
        }
        _ => {}
    }

    let (shutdown_send, mut shutdown_recv) = broadcast::channel::<()>(16);
    let shutdown_send2 = shutdown_send.clone();
    /*let mode = env::args().nth(1).unwrap().to_lowercase();
    match mode.as_str() {
        _ => {
            unimplemented!();
        }
    }*/

    let mut server = UdpServer::new("0.0.0.0:0", "stun.l.google.com:19302", shutdown_send2,1).await;
    println!("Private address is {}, Public address is {}", server.own_addr, server.pub_addr);
    tokio::spawn(async move {
        let _ = server.listen().await;
    });

    tokio::select! {
        _ = tokio::signal::ctrl_c() => {
            shutdown_send.send(()).unwrap();
        },
        _ = shutdown_recv.recv() => {},
    };
    
    return;
}

#[test]
fn test_hash_array() {
    use crypto_hash::{Algorithm, digest};
    let mut buf = [0u8; UFT_BUFFER_SIZE];
    let file_path_bytes = "test.txt".as_bytes();
    buf[0] = UftClientStatus::FILE_REQUEST as u8;
    buf[1..(1 + file_path_bytes.len())].clone_from_slice(&file_path_bytes);
    let hash = digest(Algorithm::SHA256, &buf);
    println!("Hash: {:?}\nBytes: {}", hash, hash.len());
}