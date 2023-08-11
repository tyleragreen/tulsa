use std::io::{Write, Read};
use std::net::TcpListener;
use std::thread;

pub struct Mock {}

impl Mock {
    pub fn assert(&self) -> bool {
        true
    }
}

pub struct Builder {}

impl Builder {
    pub fn with_status(&mut self, status: u16) -> &mut Builder {
        self
    }

    pub fn with_body(&mut self, body: Vec<u8>) -> &mut Builder {
        self
    }

    pub fn create(&self) -> Mock {
        Mock {}
    }
}

pub struct Server {}
use std::fs;
impl Server {
    pub fn new() -> Server {
        let address = "localhost";
        let port = 5001;
        let listener = TcpListener::bind((address, port)).unwrap();
        
        thread::spawn(move || {
            println!("server started");
            for stream in listener.incoming() {
                match stream {
                    Ok(mut stream) => {
                        println!("new connection");
                        thread::spawn(move || {
                            dbg!(&stream);
                            //let mut buffer = [0; 1024];
                            //stream.read(&mut buffer).unwrap();
                            //println!("request: {}", String::from_utf8_lossy(&buffer[..]));
                            //let data: Vec<u8> = vec![1, 2, 3];

                            let mut buffer: Vec<u8> = Vec::new();
                            let mut file = fs::File::open("fixtures/gtfs-07132023-123501").expect("Failed to open the file");
                            file.read_to_end(&mut buffer)
                                .expect("Failed to read the file");
                            let response = format!(
                                "HTTP/1.1 200 OK\r\nContent-Length: {}\r\n\r\n{}",
                                buffer.len(),
                                String::from_utf8(buffer).unwrap()
                            );
                            stream.write_all(response.as_bytes()).unwrap();
                            //stream.flush().unwrap();
                        });
                    }
                    Err(e) => {
                        eprintln!("connection failed: {}", e);
                    }
                }
            }
        });
        Server {}
    }

    pub fn mock(&self, method: &'static str, path: &'static str) -> Builder {
        Builder {}
    }

    pub fn url(&self) -> String {
        "http://localhost:5001".to_string()
    }
}
