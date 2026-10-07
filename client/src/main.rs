/**
 * October 6 2026
 * Greyson Rockwell
 * 
 * Handles all client logic, such as connecting to the server, parsing user inputs, and running user inputs
 */

use std::io;

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

#[derive(Debug)]
enum Command {
	NewUser(String, String),
	Login(String, String),
	Send(String),
	Logout,
}
fn read_input() -> Option<Command> {
	let stdin = io::stdin();

	// Read input
	let mut buffer = String::new();
	let _ = stdin.read_line(&mut buffer);
	let input = buffer.trim();
	
	let (name, rest) = input.split_once(" ").unwrap_or((input, ""));
	let args: Vec<&str> = rest.split_whitespace().collect();

	match (name.to_lowercase().as_str(), args.as_slice()) {
		("newuser", [username, password]) => Some(Command::NewUser(username.to_string(), password.to_string())),
		("login", [username, password]) => Some(Command::Login(username.to_string(), password.to_string())),
		("send", _) => Some(Command::Send(rest.trim().to_string())),
		("logout", []) => Some(Command::Logout),
		_ => None,
	}
}

#[tokio::main]
async fn main() -> Result<(), std::io::Error> {
	let server_address = get_arg_server_address().await;

	let stream = TcpStream::connect(format_addr(&server_address, SERVER_PORT)).await.expect("Failed to connect to server");
	let (mut read_stream, mut write_stream) = stream.into_split();

	println!("My chat room client. Version One.\n");


	// Reading user input
	let mut send_handle = tokio::spawn(async move  {
		loop {
			// Wait for + read input from terminal
			let command = read_input();
			if command.is_none() {
				println!("Invalid input");
				continue;
			}
			let command = command.unwrap();

			// Send a command
			// todo: handle errors from write_all's
			let msg = match command {
				Command::Send(msg) => {
					format!("send {msg}")
				},
				Command::NewUser(username, password) => {
					if username.len() < 3 || username.len() > 32 {
						println!("Username must between 3-32 characters long.");
						continue;
					}
					if password.len() < 4 || password.len() > 8 {
						println!("Password must between 4-8 characters long.");
						continue;
					}
					
					format!("newuser {username} {password}")
				},
				Command::Login(username, password) => {
					format!("login {username} {password}")
				},
				Command::Logout => "logout".to_string()
				// _ => continue 'l // ignore unimplemented commands
			};
			let _ = write_stream.write_all(msg.as_bytes()).await;
		}
	});
	let mut recv_handle = tokio::spawn(async move {
		loop {
			let mut buf = [0u8; BUFFER_SIZE];			
			let res = read_stream.read(&mut buf).await;
			if res.is_err() {
				println!("Error reading stream: {:?}", res.unwrap_err());
				continue;
			}

			let n = res.unwrap();
			if n == 0 {
				break; // peer closed connection
			}
			println!("{}", String::from_utf8_lossy(&buf[..n])); // print whatever server sent client
		}
	});

	// Wait for tasks to finish; todo: move within loop and select between sending / receiving
	tokio::select! {
		_ = &mut send_handle => {
			recv_handle.abort();
		}
		_ = &mut recv_handle => {
			send_handle.abort();
		}
	}

	Ok(())
}