#[allow(unused_imports)]
use std::net::TcpListener;
use std::{
    io::{Read, Write, Lines, BufRead, BufReader},
    net::TcpStream,
    env, fs
};

fn handle_connection(mut stream: TcpStream) {
    let mut buf_reader = BufReader::new(&mut stream);
    let mut lines = buf_reader.by_ref().lines();
    let response;

    let request_line = lines.next().unwrap().unwrap();
    let filepath = request_line.split_whitespace().nth(1).unwrap();

    if filepath == "/" {
        response = "HTTP/1.1 200 OK\r\n\r\n".to_string();
    } else if request_line.contains("/echo/") {
        let str = filepath.trim_start_matches("/echo/");
        response = format!(
            "HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\nContent-Length: {}\r\n\r\n{}",
            str.len(),
            str
        )
        .to_string();
    } else if request_line.contains("/user-agent") {
        let header = extract_headers(lines);
        response = format!(
            "HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\nContent-Length: {}\r\n\r\n{}",
            header.len(),
            header
        )
        .to_string();
    } else if filepath.starts_with("/files") {
        let file_name = filepath.trim_start_matches("/files/");
        let env_args: Vec<String> = env::args().collect();
        let mut dir = env_args[2].clone();
        dir.push_str(&file_name);
        let file = fs::read(dir);
        match file {
            Ok(fc) => {
                response = format!("HTTP/1.1 200 OK\r\nContent-Type: application/octet-stream\r\nContent-Length: {}\r\n\r\n{}\r\n", fc.len(), String::from_utf8(fc).expect("file content")).to_string();
            }
            Err(..) => response = "HTTP/1.1 404 Not Found\r\n\r\n".to_string(),
        }
    } else {
        response = "HTTP/1.1 404 Not Found\r\n\r\n".to_string();
    }
    stream.write_all(response.as_bytes()).unwrap();
}

fn extract_headers(mut lines: Lines<&mut BufReader<&mut TcpStream>>) -> String {
    let mut headers = String::new();
    for line in lines.by_ref() {
        let line = line.unwrap();
        if line == "" {
            break;
        }

        if line.to_lowercase().starts_with("user-agent:") {
            headers = line["User-Agent:".len()..].trim().to_string();
        }
    }
    headers
}

fn main() {
    let listener = TcpListener::bind("127.0.0.1:4221").unwrap();
    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                std::thread::spawn(|| handle_connection(stream));
            }
            Err(e) => {
                println!("error: {}", e);
            }
        }
    }
}


