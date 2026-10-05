use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

// const SERVER_ADDRESS: &str = "127.0.0.1";
const SERVER_PORT: u32 = 18747;
const BUFFER_SIZE: usize = 256;

fn format_addr(addr: &str, port: u32) -> String {
	addr.to_owned() + ":" + &port.to_string()
}

async fn get_arg_server_address() -> String {
	let mut args = std::env::args();
	let _ = args.next(); // discard program name

	let server_address = args.next().expect("Useage: client serverName");
	let full_addr = format_addr(&server_address, SERVER_PORT);
	let _ = tokio::net::lookup_host(&full_addr).await.expect(&format!("Host {} not found", full_addr));

	server_address
}

#[tokio::main]
async fn main() -> Result<(), std::io::Error> {
	let server_address = get_arg_server_address().await;

	println!("Connecting to {server_address}:{SERVER_PORT}...");
	let mut stream = TcpStream::connect(format_addr(&server_address, SERVER_PORT)).await.expect("Failed to connect to server");

	stream.write_all(b"hello from client").await?;

	let mut buf = [0u8; BUFFER_SIZE];
	let n = stream.read(&mut buf).await?;
	println!("Got: {}", String::from_utf8_lossy(&buf[..n]));

	Ok(())
}