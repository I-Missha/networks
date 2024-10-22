use polling::{Event, Events, Poller};
use std::net::TcpListener;

pub enum ClientState {
    // ToDo: client handshake
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let server_sock = TcpListener::bind("128.0.0.1:1080")?;

    server_sock.set_nonblocking(true)?;

    let poller = Poller::new()?;
    let key = 7;
    unsafe {
        poller.add(&server_sock, Event::readable(key))?;
    }
    let mut events = Events::new();
    loop {
        events.clear();
        let _events_number = poller.wait(&mut events, None);

        for event in events.iter() {
            if event.key == key {
                let (socket, addr) = match server_sock.accept() {
                    Ok((socket, addr)) => (socket, addr),
                    Err(e) => {
                        println!("occurr an error: {e}");
                        continue;
                    }
                };
            }
        }
    }
}
