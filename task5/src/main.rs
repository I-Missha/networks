use polling::{Event, Events, Poller};

use std::{
    io::{Read, Write},
    net::{IpAddr, Ipv4Addr, SocketAddr, TcpListener, TcpStream, ToSocketAddrs},
    time::Duration,
};
#[derive(PartialEq, Clone)]
pub enum ClientState {
    WaitingForReceiveHandshake,
    WaitForAnswerToHandshake,
    WaitingForRequest,
    WaitingForRemoteServerConnection,
    WaitingForConnection,
    WaitingForSendingConnectionStatus,
    Connected,
    StateNone,
}

pub struct Connection {
    key: usize,
    socket: TcpStream,
    dest: Option<usize>,
    state: ClientState,
}

impl Connection {
    pub fn handle_handshake(&mut self, poller: &Poller) {
        let mut buff = [0; 300];
        let _bytes_number = self.socket.read(&mut buff);
        self.state = ClientState::WaitForAnswerToHandshake;
        let _ = poller.modify(&self.socket, Event::writable(self.key));
    }
    pub fn answer_to_handshake(&mut self, poller: &Poller) {
        let mut buff = [0; 2];
        buff[0] = 5;
        buff[1] = 0;
        self.state = ClientState::WaitingForRequest;
        //println!("answer_to_handshake");
        let _bytes_number = self.socket.write(&buff);
        let _ = poller.modify(&self.socket, Event::readable(self.key));
    }
    pub fn handle_request(&mut self) -> TcpStream {
        let mut buff = [0; 256];
        let _bytes_number = self.socket.read(&mut buff).unwrap();
        self.state = ClientState::StateNone;
        // buff[2] - type of addr, 3 - domain name, 1 - ipv4
        let addr = match buff[3] {
            3 => {
                let domain_num_bytes: usize = buff[4].into();
                let domain_name = std::str::from_utf8(&buff[5..(5 + domain_num_bytes)]).unwrap();
                let mut port: u16 = 0;
                port = <u8 as Into<u16>>::into(buff[5 + domain_num_bytes]) | port;
                port = port << 8;
                port = <u8 as Into<u16>>::into(buff[6 + domain_num_bytes]) | port;
                let addrv4 = (domain_name, port.into())
                    .to_socket_addrs()
                    .expect("unable to resolve the IP address")
                    .next()
                    .expect("dns resolution returned no ip addresses");
                let addrv4 = SocketAddr::new(addrv4.ip(), port);
                dbg!(domain_name);
                addrv4
            }
            1 => {
                let addrv4 = IpAddr::V4(Ipv4Addr::new(buff[4], buff[5], buff[6], buff[7]));
                let mut port: u16 = 0;
                port = <u8 as Into<u16>>::into(buff[8]) | port;
                port = port << 8;
                port = <u8 as Into<u16>>::into(buff[9]) | port;
                let addrv4 = SocketAddr::new(addrv4, port);
                addrv4
            }
            _ => panic!("cant define type of addr"),
        };
        dbg!(addr);
        let remote_server_socket =
            TcpStream::connect_timeout(&addr, Duration::new(0, 100000000)).unwrap();
        remote_server_socket
    }
    pub fn send_ans_connected_state(&self) {}
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let server_sock = TcpListener::bind("127.0.0.1:5080")?;
    server_sock.set_nonblocking(true)?;
    let poller = Poller::new()?;
    let key = 1000000000;
    let mut connections: Vec<Connection> = Vec::new();
    let mut keys_counter = 0;
    unsafe {
        poller.add(&server_sock, Event::readable(key))?;
    }
    let mut events = Events::new();
    loop {
        events.clear();
        let _events_number = poller.wait(&mut events, None);

        for event in events.iter() {
            // append connection
            if event.key == key {
                let (socket, addr) = match server_sock.accept() {
                    Ok((socket, addr)) => (socket, addr),
                    Err(e) => {
                        println!("occurr an error: {e}");
                        continue;
                    }
                };
                socket.set_nonblocking(true)?;
                unsafe {
                    poller.add(&socket, Event::readable(keys_counter))?;
                }
                println!("new connection with: {}", addr);
                connections.push(Connection {
                    key: keys_counter,
                    dest: None,
                    socket: (socket),
                    state: ClientState::WaitingForReceiveHandshake,
                });
                keys_counter += 1;
                continue;
            }
            // read data and depending on the state do something
            let connection: &mut Connection = connections.get(event.key).unwrap();
            match connection.state {
                ClientState::WaitingForReceiveHandshake => {
                    connection.handle_handshake(&poller);
                }
                ClientState::WaitForAnswerToHandshake => {
                    connection.answer_to_handshake(&poller);
                }
                ClientState::WaitingForRequest => {
                    let remote_server_socket = connection.handle_request();
                    let client_key = connection.key;
                    unsafe {
                        let _ = poller.add(&remote_server_socket, Event::readable(keys_counter));
                    }
                    connection.dest = Some(keys_counter);
                    connection.state = ClientState::WaitingForRemoteServerConnection;
                    let _ = poller.modify(&connection.socket, Event::writable(connection.key));
                    connections.push(Connection {
                        key: keys_counter,
                        socket: remote_server_socket,
                        dest: Some(client_key),
                        state: ClientState::WaitingForConnection,
                    });
                    keys_counter += 1;
                }
                ClientState::WaitingForConnection => {}
                ClientState::WaitingForRemoteServerConnection => {
                    let save_key = connection.dest.unwrap();
                    let dest_state = connections.get(save_key).unwrap().state.clone();
                    connection.state = ClientState::WaitingForSendingConnectionStatus;
                }
                _ => continue,
            }
        }
    }
}
