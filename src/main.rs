#[allow(unused_imports)]
use std::net::TcpListener;
use std::{
    io::{Read, Write},
    net::TcpStream,
};

fn handle_connection(mut stream: TcpStream) {
    let mut buf = [0u8; 1024];

    let n = match stream.read(&mut buf) {
        Ok(0) | Err(_) => return,
        Ok(n) => n,
    };

    let request = String::from_utf8_lossy(&buf[..n]);
    let mut user_agent = "";
    for line in request.lines().skip(1) {
        if let Some((name, value)) = line.split_once(':') {
            if name.trim() == "User-Agent" {
                user_agent = value.trim();
            }
        }
    }

    println!("res: {request}");
    let path = request
        .lines()
        .next()
        .and_then(|line| line.split_whitespace().nth(1))
        .unwrap_or("");

    let response = match path {
        "/" => {
            stream.write(b"HTTP/1.1 200 OK\r\n\r\n").unwrap();
        }
        path if path.starts_with("/echo") => {
            let echo_path = path.strip_prefix("/echo/").unwrap();
            stream.write(format!("HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\nContent-Length: {}\r\n\r\n{}",echo_path.len(),echo_path).as_bytes()).unwrap();
        }
        "/user-agent" => {
            let response = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\nContent-Length: {}\r\n\r\n{}",
                user_agent.len(),
                user_agent
            );
            stream.write_all(response.as_bytes()).unwrap();
        }
        _ => {
            stream.write(b"HTTP/1.1 404 Not Found\r\n\r\n").unwrap();
        }
    };
}

fn main() {
    // You can use print statements as follows for debugging, they'll be visible when running tests.
    println!("Logs from your program will appear here!");

    let listener = TcpListener::bind("127.0.0.1:4221").unwrap();

    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                handle_connection(stream);
            }
            Err(e) => {
                println!("error: {}", e);
            }
        }
    }
}
