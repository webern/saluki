//! A local port that refuses connections and stays reserved for the life of a test.
//!
//! `panoramic-courier` must not depend on `panoramic-core`, so it keeps its own copy of the helper in
//! `panoramic-core`'s test support.

use std::net::{Ipv4Addr, SocketAddr, TcpListener, TcpStream};

use socket2::{Domain, Socket, Type};

/// A local port with no listener, so every connection to it is refused.
///
/// The port stays reserved until this is dropped. Releasing it at once would let a test running in parallel bind the
/// same ephemeral port and start answering on it.
///
/// The port is held by a socket connected to a throwaway listener. That socket only matches packets from its peer, so
/// connections from anywhere else are refused. A socket that is bound but not listening would hold the port too, but
/// macOS silently drops connections to it instead of refusing them.
///
/// Windows retries a refused connection for about two seconds before failing.
pub struct ClosedPort {
    port: u16,
    _holder: Socket,
    _peer: TcpStream,
}

impl ClosedPort {
    /// Returns the port number.
    pub fn port(&self) -> u16 {
        self.port
    }
}

/// Reserves a local port with no listener. See [`ClosedPort`].
pub fn closed_port() -> ClosedPort {
    let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).expect("should bind a listener");
    // Without `SO_REUSEADDR`, which `socket2` doesn't set, Linux won't let any other socket bind the port.
    let holder = Socket::new(Domain::IPV4, Type::STREAM, None).expect("should create a socket");
    holder
        .bind(&SocketAddr::from((Ipv4Addr::LOCALHOST, 0)).into())
        .expect("should bind an ephemeral port");
    holder
        .connect(&listener.local_addr().expect("should have a local address").into())
        .expect("should connect to the listener");
    let (peer, _) = listener.accept().expect("should accept the connection");
    let port = holder
        .local_addr()
        .ok()
        .and_then(|addr| addr.as_socket())
        .expect("should have a local address")
        .port();
    ClosedPort {
        port,
        _holder: holder,
        _peer: peer,
    }
}

#[cfg(test)]
mod tests {
    use std::io::ErrorKind;

    use super::*;

    #[test]
    fn a_closed_port_refuses_connections_and_stays_reserved() {
        let closed = closed_port();
        let other = closed_port();
        assert_ne!(closed.port(), other.port());

        let error = TcpStream::connect((Ipv4Addr::LOCALHOST, closed.port())).expect_err("nothing should listen");
        assert_eq!(error.kind(), ErrorKind::ConnectionRefused);
        // `std` listeners set `SO_REUSEADDR`, which lets them share the port, so bind a plain socket as `closed_port`
        // does.
        let rival = Socket::new(Domain::IPV4, Type::STREAM, None).expect("should create a socket");
        assert!(
            rival
                .bind(&SocketAddr::from((Ipv4Addr::LOCALHOST, closed.port())).into())
                .is_err(),
            "the port should stay reserved while the guard lives"
        );
    }
}
