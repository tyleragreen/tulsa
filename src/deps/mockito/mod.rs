use std::io::Write;
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

impl Server {
    pub fn new() -> Server {
        let address = "127.0.0.1";
        let port = 5001;
        let listener = TcpListener::bind((address, port)).unwrap();
        
        thread::spawn(move || {
            println!("server started");
            for stream in listener.incoming() {
                match stream {
                    Ok(mut stream) => {
                        println!("new connection");
                        thread::spawn(move || {
                            let data: Vec<u8> = vec![1, 2, 3];
                            let response = format!(
                                "HTTP/1.1 200 OK\r\nContent-Length: {}\r\n\r\n{}",
                                data.len(),
                                String::from_utf8(data).unwrap()
                            );
                            stream.write_all(response.as_bytes()).unwrap();
                            stream.flush().unwrap();
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
        "http://127.0.0.1:5001".to_string()
    }
}
