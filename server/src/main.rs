/**
 * October 6 2026
 * Greyson Rockwell
 * 
 * Handles all server logic, such as creating the socket, reading + parsing data from the socket, saving user info, and emitting messages back to clients
 */
use std::sync::Arc;
use std::io::{Error, ErrorKind};
use std::{error, fmt};
use std::fmt::Display;

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


#[derive(Debug)]
struct SendError {
	v: String,
}
impl SendError {
	fn new() -> Self {
		SendError { v: "Something went wrong".to_string() }
	}
}
impl error::Error for SendError {}
impl Display for SendError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "SendError: {}", self.v)
    }
}

async fn handle_client(stream: TcpStream, users: Arc<Users>) -> std::io::Result<()> {
	let conn_username; // this functions as a unique user ID
	let conn_passwd;
	let (mut read_stream, mut write_stream) = stream.into_split();

	// Prelogin: limited set of functionality, as you won't have a User account in the system. It's a bit redundant, but necessary
	'l: loop {
		let mut buffer = [0u8; BUFFER_SIZE];
		let n = read_stream.read(&mut buffer).await?;
		if n == 0 { return Ok(()); } // peer closed the connection (EOF)

		let text = String::from_utf8_lossy(&buffer[..n]).into_owned();
		let command = parse_command(&text);
		if command.is_none() { continue; }
		let command = command.unwrap();

		use Command::*;
		match command {
			NewUser(username, password) => 'm: {
				if users.contains(&username) {
					let _ = write_stream.write_all("Denied. User account already exists.".as_bytes()).await;
					break 'm;
				}

				users.insert(&username, &password, None);
				let _ = write_stream.write_all("New user account created. Please login.".as_bytes()).await;
				println!("New user account created");
			},
			Login(username, password) => 'm: {
				if !users.contains(&username) {
					let _ = write_stream.write_all("Denied. User name or password incorrect.".as_bytes()).await;
					break 'm;
				}
				
				let user = users.get(&username).expect("Something went horribly wrong! Check Users::contains()");
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

				users.insert(&username, &password, Some(write_stream)); // note: this takes a lock on Users; be careful of deadlock
				conn_username = username;
				conn_passwd = password;
				break 'l;
			},
			Logout|Send(_, _) => {
				let _ = write_stream.write_all("Denied. Please login first.".as_bytes()).await;
			},
		};
	}

	// After login: Full command access
	'l: loop {
		let mut buffer = [0u8; BUFFER_SIZE];
		let n = read_stream.read(&mut buffer).await?;
		if n == 0 { return Ok(()); } // peer closed the connection (EOF)

		let text = String::from_utf8_lossy(&buffer[..n]).into_owned();
		let command = parse_command(&text);
		if command.is_none() { continue; }
		let command = command.unwrap();


		// Helper methods for sending messages
		let get_write_stream = |username: &str| {
			let cur_user = users.get(&username).unwrap().clone(); // User is currently logged in, so unwraps shouldn't fail
			let write_stream = cur_user.socket_write.clone();
			write_stream
		};
		let send_msg = async |username: &str, msg: &str| -> Result<(), SendError> {
			let write_stream = match get_write_stream(&username) {
				Some(v) => v,
				None => { return Result::Err(SendError::new()) }
			};
			let mut write_stream = write_stream.lock().expect(&format!("Failed to get lock on target user ({})", username)); // expect will only fail if mutex is poisoned, which is bad enough to stop thread
			let result = write_stream.write_all(format!("{}: {}", conn_username, msg).as_bytes()).await;
			match result {
				Ok(_) => Ok(()),
				Err(_) => Result::Err(SendError::new())
			}			
		};
		let broadcast_msg = async |msg: &str| {
			users.iter_keys(async |username| {
				send_msg(username, msg).await; // if there's any error, just move on to next client
			}).await;
		};

		use Command::*;
		match command {
			Send(target, msg) => {
				// todo: send to target
				// "send to target" is doing a lot of heavy lifting here because that's the entire difficult part of V2
				
				match target {
					SendTarget::All => {
						broadcast_msg(&msg);
						println!("{} (to all): {}", conn_username, msg);
					}
					SendTarget::User(target_username) => {
						if !users.contains(&target_username) {
							if send_msg(&conn_username, "Invalid username. User may be offline or misspelled.").await.is_err() {
								break 'l; // closed connection
							};
							continue 'l;
						}

						if send_msg(&target_username, &format!("{}: {}", conn_username, msg)).await.is_err() {
							break 'l; // closed connection
						};
						println!("{} (to {}): {}", conn_username, target_username, msg);
					}
				}
			},
			Logout => {
				// V1: don't need to worry about cleaning up pipes or anything, just kick out of loop + end connection
				broadcast_msg(&format!("{} left.", conn_username));
				println!("{} logout", conn_username);
				break 'l;
			},
			Login(_, _)|NewUser(_, _) => {
				if send_msg(&conn_username, "Denied. Please log out first.").await.is_err() {
					break 'l; // closed connection
				};
			},
		};
	}


	// Clear connection from user
	users.insert(&conn_username, &conn_passwd, None);

	Ok(())
}