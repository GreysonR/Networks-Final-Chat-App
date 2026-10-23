/**
 * October 23 2026
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

	println!("My chat room server. Version Two.\n");
	
	handle_requests(users).await;

	Ok(())
}

// Waits for connections, then creates a new thread for each connection and hands responsibility off to handle_client
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

// Handles everything for a single connection: Reading & responding to input
async fn handle_client(stream: TcpStream, users: Arc<Users>) -> std::io::Result<()> {
	let conn_username;
	let conn_passwd;
	let (mut read_stream, mut write_stream) = stream.into_split();

	// Pre-login
	'l: loop {
		let mut buffer = [0u8; BUFFER_SIZE];
		let n = read_stream.read(&mut buffer).await?;
		if n == 0 { return Ok(()); }

		let text = String::from_utf8_lossy(&buffer[..n]).into_owned();
		let Some(command) = parse_command(&text) else { continue };

		use Command::*;
		match command {
			NewUser(username, password) => 'm: {
				if users.contains(&username).await {
					let _ = write_stream.write_all("Denied. User account already exists.".as_bytes()).await;
					break 'm;
				}
				users.insert(&username, &password, None).await;
				let _ = write_stream.write_all("New user account created. Please login.".as_bytes()).await;
				println!("New user account created");
			},
			Login(username, password) => 'm: {
				if !users.contains(&username).await {
					let _ = write_stream.write_all("Denied. User name or password incorrect.".as_bytes()).await;
					break 'm;
				}
				let user = users.get(&username).await.expect("Check Users::contains()");
				if user.password != password {
					let _ = write_stream.write_all("Denied. User name or password incorrect.".as_bytes()).await;
					break 'm;
				}
				if user.socket_write.is_some() {
					let _ = write_stream.write_all("Denied. User is already logged in.".as_bytes()).await;
					break 'm;
				}
				
				let _ = write_stream.write_all("login confirmed".as_bytes()).await;
				println!("{} login", username);

				users.broadcast(&format!("{} joins.", username), None).await; // need to broadcast this before connecting the new user so the new user doesn't get it
				users.insert(&username, &password, Some(write_stream)).await;

				conn_username = username;
				conn_passwd = password;

				break 'l;
			},
			Logout | Send(_, _) => {
				let _ = write_stream.write_all("Denied. Please login first.".as_bytes()).await;
			},
		};
	}

	// After login
	'l: loop {
		let mut buffer = [0u8; BUFFER_SIZE];
		let n = read_stream.read(&mut buffer).await?;
		if n == 0 { break 'l; } // peer disconnected

		let text = String::from_utf8_lossy(&buffer[..n]).into_owned();
		let Some(command) = parse_command(&text) else { continue };

		use Command::*;
		match command {
			Send(SendTarget::All, msg) => {
				users.broadcast(&format!("{conn_username}: {msg}"), Some(&conn_username)).await;
				println!("{conn_username}: {msg}");
			}
			Send(SendTarget::User(target), msg) => {
				if !users.contains(&target).await {
					if users.send(&conn_username, "Invalid username. User may be offline or misspelled.").await.is_err() {
						break 'l; // closed connection
					}
					continue 'l;
				}
				if users.send(&target, &format!("{conn_username}: {msg}")).await.is_err() {
					break 'l; // closed connection
				}
				println!("{conn_username} (to {target}): {msg}");
			}
			Logout => {
				break 'l;
			}
			Login(_, _) | NewUser(_, _) => {
				if users.send(&conn_username, "Denied. Please log out first.").await.is_err() {
					break 'l; // closed connection
				}
			}
		};
	}

	// Clear connection from user
	println!("{conn_username} logout");
	users.insert(&conn_username, &conn_passwd, None).await;
	users.broadcast(&format!("{conn_username} left."), None).await;
	Ok(())
}