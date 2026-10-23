/**
 * October 23 2026
 * Greyson Rockwell
 * 
 * Handles all client logic, such as connecting to the server, parsing user inputs, and running user inputs
 */

use std::{io, sync::{Arc, Mutex}};

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

mod parser;
use parser::*;

// const SERVER_ADDRESS: &str = "127.0.0.1";
const SERVER_PORT: u32 = 18747;
const BUFFER_SIZE: usize = 256;

fn format_addr(addr: &str, port: u32) -> String {
	addr.to_owned() + ":" + &port.to_string()
}

async fn get_arg_server_address() -> String {
	let mut args = std::env::args();
	let _ = args.next(); // discard program name

	let server_address = args.next().expect("Usage: client serverName");
	let full_addr = format_addr(&server_address, SERVER_PORT);
	let _ = tokio::net::lookup_host(&full_addr).await.expect(&format!("Host {} not found", full_addr));

	server_address
}



// Reads input from stdin and parses it into a command.
// This is a blocking operation unfortunately
// Returns: Some(Command), or None if it fails to parse the input.
fn read_input() -> Option<Command> {
	let stdin = io::stdin();

	let mut buffer = String::new();
	let _ = stdin.read_line(&mut buffer);
	
	parse_command(&buffer)
}

#[tokio::main]
async fn main() -> Result<(), std::io::Error> {
	let server_address = get_arg_server_address().await;

	let stream = TcpStream::connect(format_addr(&server_address, SERVER_PORT)).await.expect("Failed to connect to server");
	let (mut read_stream, mut write_stream) = stream.into_split();

	println!("My chat room client. Version Two.\n");

	// Disconnected flag, which alerts each thread if the other has stopped. 
	let disconnected = Arc::new(Mutex::new(false));
	let disconnected_send = Arc::clone(&disconnected);
	let disconnected_recv = Arc::clone(&disconnected);


	// Reading user input & sending to server
	let mut send_handle = tokio::spawn(async move  {
		loop {
			if *disconnected_send.lock().unwrap() == true { break; }

			// Wait for + read input from terminal
			let command = read_input(); // this is a problematic blocking operation
			if *disconnected_send.lock().unwrap() == true { break; } // don't print error message after user input if disconnected. Very hacky, but technically works
			if command.is_none() {
				println!("Invalid input");
				continue;
			}
			let command = command.unwrap();

			// Send the command. What's sent winds up being a reconstruction whatever was typed, but parsing is necessary for client-side input validation.
			let msg = match command {
				Command::Send(target, msg) => {
					format!("send {target} {msg}")
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
			};
			let result = write_stream.write_all(msg.as_bytes()).await;
			if result.is_err() {
				println!("Disconnected. Enter to exit. ");
				break; // server closed connection
			}
		}
	});
	
	// Printing server responses
	let mut recv_handle = tokio::spawn(async move {
		loop {
			if *disconnected_recv.lock().unwrap() == true {
				break;
			}

			let mut buf = [0u8; BUFFER_SIZE];			
			let res = read_stream.read(&mut buf).await;

			match res {
				Ok(0) => { // server closed connection
					println!("Disconnected. Enter to exit. ");
					break;
				}
				Ok(n) => { // print whatever server sent client
					println!("{}", String::from_utf8_lossy(&buf[..n])); 
				}
				Err(e) => {
					println!("Error reading stream: {}", e); // hopefully this never happens
				}
			}
		}
	});

	// Wait for tasks to finish & try to close the other when one finishes first
	// Problem: send_handle reads user input in a blocking way, so it can't exit until the user inputs something, requiring disconnected flag. However, this isn't worth fixing.
	tokio::select! {
		_ = &mut send_handle => {
			*disconnected.lock().unwrap() = true;
			recv_handle.abort();
		}
		_ = &mut recv_handle => {
			*disconnected.lock().unwrap() = true;
			send_handle.abort();
		}
	}

	Ok(())
}