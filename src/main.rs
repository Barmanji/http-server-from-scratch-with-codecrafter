#[allow(unused_imports)]
use flate2::{write::GzEncoder, Compression};
use std::{
    env, fs,
    io::{BufRead, BufReader, Read, Write},
    net::{TcpListener, TcpStream},
    path::{Path, PathBuf},
    thread,
};

fn main() {
    let dir = parse_directory();
    let listener = TcpListener::bind("127.0.0.1:4221").unwrap();

    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                let dir = dir.clone();
                thread::spawn(move || handle_connection(stream, dir));
            }
            Err(e) => println!("error: {}", e),
        }
    }
}

// Finds the value after `--directory` in the command-line arguments.
fn parse_directory() -> Option<String> {
    let args: Vec<String> = env::args().collect();
    let i = args.iter().position(|a| a == "--directory")?;
    args.get(i + 1).cloned()
}

// Builds dir/name, refusing names that could escape the directory.
fn safe_path(dir: &Option<String>, name: &str) -> Option<PathBuf> {
    let dir = dir.as_ref()?;
    if name.is_empty() || name.contains("..") || name.contains('/') {
        return None;
    }
    Some(Path::new(dir).join(name))
}

fn not_found() -> Vec<u8> {
    b"HTTP/1.1 404 Not Found\r\n\r\n".to_vec()
}

fn text_response(body: &str, gzip: bool) -> Vec<u8> {
    if gzip {
        // Compress the body into raw bytes.
        let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
        encoder.write_all(body.as_bytes()).unwrap();
        let compressed = encoder.finish().unwrap();

        // Headers as text; Content-Length is the COMPRESSED size.
        let mut resp = format!(
            "HTTP/1.1 200 OK\r\nContent-Encoding: gzip\r\nContent-Type: text/plain\r\nContent-Length: {}\r\n\r\n",
            compressed.len()
        )
        .into_bytes();

        // Body is appended as raw bytes, never put through format!.
        resp.extend_from_slice(&compressed);
        resp
    } else {
        format!(
            "HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\nContent-Length: {}\r\n\r\n{}",
            body.len(),
            body
        )
        .into_bytes()
    }
}

fn handle_connection(mut stream: TcpStream, dir: Option<String>) {
    // Reader works on a clone of the socket, so we can still write to `stream`.
    let mut reader = BufReader::new(stream.try_clone().unwrap());

    // 1. Request line: "POST /files/abc HTTP/1.1"
    let mut request_line = String::new();
    if reader.read_line(&mut request_line).unwrap_or(0) == 0 {
        return; // client closed without sending anything
    }
    let mut parts = request_line.split_whitespace();
    let method = parts.next().unwrap_or("");
    let path = parts.next().unwrap_or("");

    // 2. Headers: read line by line until the blank line.
    let mut user_agent = String::new();
    let mut content_length: usize = 0;
    let mut accept_encoding = String::new();

    loop {
        let mut line = String::new();
        if reader.read_line(&mut line).unwrap_or(0) == 0 {
            break;
        }
        let line = line.trim_end(); // removes the trailing \r\n
        if line.is_empty() {
            break; // blank line = end of headers; body (if any) starts next
        }
        if let Some((name, value)) = line.split_once(':') {
            let value = value.trim();
            if name.eq_ignore_ascii_case("user-agent") {
                user_agent = value.to_string();
            } else if name.eq_ignore_ascii_case("content-length") {
                content_length = value.parse().unwrap_or(0);
            } else if name.eq_ignore_ascii_case("accept-encoding") {
                accept_encoding = value.to_string();
            }
        }
    }
    let gzip_ok = accept_encoding.split(',').any(|s| s.trim() == "gzip");

    // 3. Route on (method, path).
    let response: Vec<u8> = match (method, path) {
        ("GET", "/") => b"HTTP/1.1 200 OK\r\n\r\n".to_vec(),

        ("GET", "/user-agent") => text_response(&user_agent, false),

        ("GET", p) if p.starts_with("/echo/") => {
            text_response(p.strip_prefix("/echo/").unwrap(), gzip_ok)
        }

        ("GET", p) if p.starts_with("/files/") => {
            let name = p.strip_prefix("/files/").unwrap();
            match safe_path(&dir, name).map(fs::read) {
                Some(Ok(bytes)) => {
                    let mut resp = format!(
                        "HTTP/1.1 200 OK\r\nContent-Type: application/octet-stream\r\nContent-Length: {}\r\n\r\n",
                        bytes.len()
                    )
                    .into_bytes();
                    resp.extend_from_slice(&bytes); // body stays raw bytes
                    resp
                }
                _ => not_found(),
            }
        }

        ("POST", p) if p.starts_with("/files/") => {
            let name = p.strip_prefix("/files/").unwrap();
            match safe_path(&dir, name) {
                Some(full_path) => {
                    // Read exactly Content-Length bytes from the same reader.
                    let mut body = vec![0u8; content_length];
                    if reader.read_exact(&mut body).is_ok() && fs::write(full_path, &body).is_ok() {
                        b"HTTP/1.1 201 Created\r\n\r\n".to_vec()
                    } else {
                        b"HTTP/1.1 500 Internal Server Error\r\n\r\n".to_vec()
                    }
                }
                None => not_found(),
            }
        }

        _ => not_found(),
    };

    let _ = stream.write_all(&response);
}
