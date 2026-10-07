/**
 * October 6 2026
 * Greyson Rockwell
 * 
 * Handles all server logic, such as creating the socket, reading + parsing data from the socket, saving user info, and emitting messages back to clients
 */
use std::sync::Arc;
use tokio::{io::{AsyncReadExt, AsyncWriteExt}, net::{TcpSocket, TcpListener, TcpStream}};

mod db;
use db::*;

mod parser;
use parser::*;

const SERVER_ADDRESS: &str = "127.0.0.1";
const SERVER_PORT: u32 = 18747;
const BUFFER_SIZE: usize = 512;
const MAX_PENDING: u32 = 5;

const USER_FILENAME: &str = "users.txt";


fn create_socket(address: &str, port: u32) -> Result<TcpListener, std::io::Error> {
	let socket = TcpSocket::new_v4()?;
	socket.set_keepalive(true)?;
	socket.set_reuseport(true)?;
	let full_addr = address.to_owned() + ":" + &port.to_string();
	socket.bind(full_addr.parse()
		.expect(&format!("Invalid address: {}", &full_addr))
	)?;

	let listener = socket.listen(MAX_PENDING)?;

	Ok(listener)
}

#[tokio::main]
async fn main() -> Result<(), std::io::Error> {
	let users = Users::new(USER_FILENAME);

	println!("My chat room server. Version One.\n");
	
	handle_requests(users).await;

	Ok(())
}

async fn handle_requests(users: Users) {
	let users = Arc::new(users);
	let listener = create_socket(SERVER_ADDRESS, SERVER_PORT).expect("Failed to create socket");
	
	loop {
		let (stream, peer) = listener.accept().await.expect("Failed to get client");
		let users = Arc::clone(&users);
		tokio::spawn(async move {
			if let Err(err) = handle_client(stream, users).await {
				eprintln!("{peer} error: {err}");
			}
		});
	}
}


async fn handle_client(mut stream: TcpStream, users: Arc<Users>) -> std::io::Result<()> {
	// Prelogin: limited set of functionality, as you won't have a User account in the system. It's a bit redundant, but necessary
	loop {
		let mut buffer = [0u8; BUFFER_SIZE];
		let n = stream.read(&mut buffer).await?;
		if n == 0 { return Ok(()); } // peer closed the connection (EOF)

		let text = String::from_utf8_lossy(&buffer[..n]).into_owned();
		let command = parse_command(&text);
		if command.is_none() { continue; }
		let command = command.unwrap();

		use Command::*;
		match command {
			NewUser(username, password) => 'm: {
				if users.contains(&username) {
					let _ = stream.write_all("Denied. User account already exists.".as_bytes()).await;
					break 'm;
				}

				users.insert(&username, &password);
				let _ = stream.write_all("New user account created. Please login.".as_bytes()).await;
				println!("New user account created");
			},
			Login(username, password) => 'm: {
				if !users.contains(&username) {
					let _ = stream.write_all("Denied. User name or password incorrect.".as_bytes()).await;
					break 'm;
				}
				
				let user = users.get(&username).expect("Something went horribly wrong! Check Users::contains()");
				if user.password != password {
					let _ = stream.write_all("Denied. User name or password incorrect.".as_bytes()).await;
					break 'm;
				}
				
				let _ = stream.write_all("login confirmed".as_bytes()).await;
				println!("{} login", username);
				// todo: actually log in
			},
			Logout|Send(_) => {
				let _ = stream.write_all("Denied. Please login first.".as_bytes()).await;
			},
		}
	}

	// After login: Full command access
	// loop {}
}