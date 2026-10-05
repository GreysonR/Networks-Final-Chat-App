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
#[allow(unused)] // todo: remove this
enum Command {
	NewUser(String, String),
	Login(String, String),
	Send(String),
	Logout,
}
fn read_input() -> Command {
	let stdin = io::stdin();
	loop {
		// Read input
		let mut buffer = String::new();
		let _ = stdin.read_line(&mut buffer);
		let input = buffer.trim();
		
		// Split input by spaces
		let mut command_args = input.split(" ");
		
		// Read command name
		let command_name = command_args.next();
		if command_name.is_none() { continue; }
		let command_name = command_name.unwrap().to_lowercase();

		match command_name.as_str() {
			"newuser" => 'newuser: {
				let username = command_args.next();
				if username.is_none() { break 'newuser; }
				let password = command_args.next();
				if password.is_none() { break 'newuser; }
				return Command::NewUser(username.unwrap().to_string(), password.unwrap().to_string());
			},
			"login" => 'login: {
				let username = command_args.next();
				if username.is_none() { break 'login; }
				let password = command_args.next();
				if password.is_none() { break 'login; }
				return Command::Login(username.unwrap().to_string(), password.unwrap().to_string());
			},
			"send" => 'send: {
				let msg = command_args.collect::<Vec<_>>().join(" ");
				if msg.len() == 0 { break 'send; }
				return Command::Send(msg.to_string());
			},
			"logout" => {
				return Command::Logout;
			},
			_ => ()
		};

		println!("Invalid input"); // todo: add instructions for possible commands
	}
}

#[tokio::main]
async fn main() -> Result<(), std::io::Error> {
	let server_address = get_arg_server_address().await;

	// println!("Connecting to {server_address}:{SERVER_PORT}...");
	let mut stream = TcpStream::connect(format_addr(&server_address, SERVER_PORT)).await.expect("Failed to connect to server");
	// println!("Connected.");

	println!("My chat room client. Version One.\n");


	loop {
		// Wait for + read input from terminal
		let command = read_input();

		// Send a command
		match command {
			Command::Send(msg) => {
				println!("Sending {msg}");

				stream.write_all(msg.as_bytes()).await?;
			},
			Command::Logout => {
				break; // todo: allow to still receive final logout message from server
			}
			_ => continue
		};

		// Wait for reply
		let mut buf = [0u8; BUFFER_SIZE];
		let n = stream.read(&mut buf).await?;
		println!("Got: {}", String::from_utf8_lossy(&buf[..n]));
	}

	Ok(())
}