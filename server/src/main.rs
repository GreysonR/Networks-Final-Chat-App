use tokio::{io::{AsyncReadExt, AsyncWriteExt}, net::{TcpSocket, TcpListener, TcpStream}};

const SERVER_ADDRESS: &str = "127.0.0.1";
const SERVER_PORT: u32 = 18747;
const BUFFER_SIZE: usize = 256;

fn create_socket(address: &str, port: u32) -> Result<TcpListener, std::io::Error> {
	let socket = TcpSocket::new_v4()?;
	socket.set_keepalive(true)?;
	let full_addr = address.to_owned() + ":" + &port.to_string();
	socket.bind(full_addr.parse()
		.expect(&format!("Invalid address: {}", &full_addr))
	)?;

	let listener = socket.listen(port)?;

	Ok(listener)
}

#[tokio::main]
async fn main() -> Result<(), std::io::Error> {
	println!("Creating socket at {SERVER_ADDRESS}:{SERVER_PORT}...");
	let listener = create_socket(SERVER_ADDRESS, SERVER_PORT).expect("Failed to create socket");
	println!("Socket created. Listening for connections...");

	loop {
		let (stream, peer) = listener.accept().await.expect("Failed to get client");

		tokio::spawn(async move {
			if let Err(err) = handle_client(stream).await {
				eprintln!("{peer} error: {err}");
			}

			println!("Disconnected: {peer}");
		});
	}



	// Ok(()) // listener is automatically closed when it's dropped
}

async fn handle_client(mut stream: TcpStream) -> std::io::Result<()> {
	let mut buffer = [0u8; BUFFER_SIZE];
	
	println!("Connected: {}", stream.peer_addr().expect("Failed to get peer address"));

	loop {
		let n = stream.read(&mut buffer).await?;
		if n == 0 {
			return Ok(()); // peer closed the connection (EOF)
		}
		// process buf[..n]
		stream.write_all(&buffer[..n]).await?;
	}
}