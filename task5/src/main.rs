use polling::{Event, Events, Poller};
use socket2::SockAddr;

use std::{
    fmt::write,
    hash::Hash,
    io::{Read, Write},
    net::{IpAddr, Ipv4Addr, SocketAddr, TcpListener, TcpStream, ToSocketAddrs},
    time::Duration,
};

#[derive(PartialEq, Clone, Debug)]
pub enum ClientState {
    WaitingForReceiveHandshake,
    WaitForAnswerToHandshake,
    WaitingForRequest,
    WaitingForRemoteServerConnection,
    WaitingForSendingConnectionStatus,
    Connected,
    StateNone,
}

#[derive(Debug)]
pub struct Connection {
    key: usize,
    socket: TcpStream,
    dest: Option<usize>,
    state: ClientState,
    is_ready_to_send: bool,
    buffer_to_send: Vec<u8>,
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
    pub fn handle_request(&mut self) -> Option<TcpStream> {
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
            4 => {
                return None;
            }
            _ => return None,
        };
        dbg!(addr);
        let remote_server_socket = TcpStream::connect_timeout(&addr, Duration::new(2, 0)).unwrap();
        let _ = remote_server_socket.set_nonblocking(true);
        Some(remote_server_socket)
    }

    pub fn get_response(&mut self) -> Vec<u8> {
        let mut buff = [0; 300];
        let bytes_number = self.socket.read(&mut buff).unwrap();
        dbg!(&buff[0..bytes_number]);
        Vec::from(&buff[0..bytes_number])
    }
}

fn get_dest_state(connections: &Vec<Connection>, key: usize) -> Option<ClientState> {
    let connection: &Connection = connections.get(key).unwrap();
    let dest_key = connection.dest;
    let dest_connection = match dest_key {
        Some(key_tmp) => connections.get(key_tmp),
        None => None,
    };
    match dest_connection {
        Some(connection) => Some(connection.state.clone()),
        None => None,
    }
}

fn get_dest_addr(connections: &Vec<Connection>, key: usize) -> Option<SocketAddr> {
    let connection: &Connection = &connections.get(key).unwrap();
    let dest_key = connection.dest;
    let dest_connection = match dest_key {
        Some(key_tmp) => connections.get(key_tmp),
        None => None,
    };
    match dest_connection {
        Some(connection) => Some(connection.socket.local_addr().unwrap()),
        None => None,
    }
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
                    is_ready_to_send: false,
                    buffer_to_send: Vec::new(),
                });
                keys_counter += 1;
                let _ = poller.modify(&server_sock, Event::readable(key));
                continue;
            }
            // read data and depending on the state do something
            let connection: &Connection = connections.get(event.key).unwrap();
            let dest_key = connection.dest;
            let dest_addr = get_dest_addr(&connections, event.key);
            let connection: &mut Connection = connections.get_mut(event.key).unwrap();
            match connection.state {
                ClientState::WaitingForReceiveHandshake => {
                    connection.handle_handshake(&poller);
                }
                ClientState::WaitForAnswerToHandshake => {
                    connection.answer_to_handshake(&poller);
                }
                ClientState::WaitingForRequest => {
                    let remote_server_socket = match connection.handle_request() {
                        Some(socket) => socket,
                        None => {
                            poller.delete(&connection.socket);
                            continue;
                        }
                    };
                    let client_key = connection.key;
                    unsafe {
                        let _ = poller.add(&remote_server_socket, Event::readable(keys_counter));
                    }
                    connection.dest = Some(keys_counter);
                    connection.state = ClientState::WaitingForSendingConnectionStatus;
                    let _ = poller.modify(&connection.socket, Event::writable(connection.key));
                    unsafe {
                        let _ = poller.add(&remote_server_socket, Event::writable(keys_counter));
                    }
                    connections.push(Connection {
                        key: keys_counter,
                        socket: remote_server_socket,
                        dest: Some(client_key),
                        state: ClientState::Connected,
                        is_ready_to_send: false,
                        buffer_to_send: Vec::new(),
                    });
                    keys_counter += 1;
                }
                ClientState::WaitingForSendingConnectionStatus => {
                    let dest_ip = match connection.socket.local_addr().unwrap().ip() {
                        IpAddr::V4(ip) => ip.octets(),
                        IpAddr::V6(_) => panic!("how i got ipv6"),
                    };
                    let dest_port = connection.socket.local_addr().unwrap().port().to_le_bytes();
                    dbg!(dest_addr);
                    let mut data_to_send: [u8; 10] = [0; 10];
                    data_to_send[0] = 5;
                    data_to_send[1] = 0;
                    data_to_send[2] = 0;
                    data_to_send[3] = 1;

                    data_to_send[4] = dest_ip[0];
                    data_to_send[5] = dest_ip[1];
                    data_to_send[6] = dest_ip[2];
                    data_to_send[7] = dest_ip[3];

                    data_to_send[8] = dest_port[0];
                    data_to_send[9] = dest_port[1];
                    let _bytes_number = connection.socket.write(&data_to_send);
                    let _ = poller.modify(&connection.socket, Event::readable(connection.key));
                    connection.state = ClientState::Connected;
                }
                ClientState::Connected => {
                    if event.readable {
                        let mut buff = [0; 1024];
                        let bytes_number = connection.socket.read(&mut buff)?;
                        let _ = poller.modify(&connection.socket, Event::readable(connection.key));
                        connections
                            .get_mut(dest_key.unwrap())
                            .unwrap()
                            .is_ready_to_send = true;
                        connections
                            .get_mut(dest_key.unwrap())
                            .unwrap()
                            .buffer_to_send = Vec::from(&buff[0..bytes_number]);
                        let bytes = connections
                            .get_mut(dest_key.unwrap())
                            .unwrap()
                            .socket
                            .write(&buff[0..bytes_number]);
                        /*println!(
                            "source key is: {}\nand dest key is: {}\n bytes number: {}",
                            event.key,
                            dest_key.unwrap(),
                            bytes?
                        );*/
                    } /*else if event.writable && connection.is_ready_to_send {
                          let _bytes_number = connection.socket.write(&connection.buffer_to_send);
                          connection.is_ready_to_send = false;
                          let _ = poller.modify(&connection.socket, Event::all(connection.key));
                      }*/
                }
                _ => continue,
            }
        }
    }
}
