use std::{
    io::{self, BufRead, BufReader, Write},
    net::{Shutdown, TcpListener, TcpStream},
    thread,
};

fn main() {
    let listener = TcpListener::bind("127.0.0.1:8080").unwrap();

    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                if let Err(e) = handle_connection(stream) {
                    eprintln!("{e}");
                }
            }
            Err(error) => eprintln!("error read stream: {error}"),
        }
    }
}

fn handle_connection(mut stream: TcpStream) -> io::Result<()> {
    let mut reader = BufReader::new(stream);
    let mut buffer = Vec::new();

    let n = reader.read_until(b'\n', &mut buffer)?;
    if n == 0 {
        return Err(io::Error::new(
            io::ErrorKind::UnexpectedEof,
            "error read 0 bytes",
        ));
    }

    // CONNECT example.com:443 HTTP/1.1\r\n
    if !buffer.starts_with(b"CONNECT ") {
        return Err(io::Error::new(
            io::ErrorKind::Interrupted,
            "error Interrupted",
        ));
    }

    let rest = &buffer[b"CONNECT ".len()..];

    // example.com:443 HTTP/1.1\r\n
    let host_port_end = rest
        .iter()
        .position(|b: &u8| b == &b' ')
        .unwrap_or(rest.len());
    let host_port = &rest[..host_port_end];

    let host_port_str = std::str::from_utf8(host_port)
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "host:port not UTF-8"))?
        .trim_end();

    if host_port_str.is_empty() {
        return Err(io::Error::new(
            io::ErrorKind::Interrupted,
            "error host port empty",
        ));
    }

    println!("{host_port_str}");

    // finally connection
    let mut target = TcpStream::connect(host_port_str)?;

    let mut local_conn = reader.get_mut();
    local_conn.write_all(b"HTTP/1.1 200 Connection Established\r\n\r\n")?;
    local_conn.flush()?;

    let mut client = reader.get_ref().try_clone()?;
    let mut remote = target.try_clone()?;
    let mut client2 = client.try_clone()?;

    // Proxying
    let c2u = thread::spawn(move || {
        let r = io::copy(&mut client, &mut remote);
        let _ = remote.shutdown(Shutdown::Write);
        r
    });

    let u2c = thread::spawn(move || {
        let r = io::copy(&mut target, &mut client2);
        let _ = client2.shutdown(Shutdown::Write);
        r
    });

    c2u.join().unwrap()?;
    u2c.join().unwrap()?;

    Ok(())
}
