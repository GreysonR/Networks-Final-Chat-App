/**
 * October 6 2026
 * Greyson Rockwell
 * 
 * Parses messages sent by clients into commands for the server
 */

#[derive(Debug)]
pub enum SendTarget {
	All,
	User(String),
}

#[derive(Debug)]
pub enum Command {
	NewUser(String, String),
	Login(String, String),
	Send(SendTarget, String),
	Logout,
}

pub fn parse_command(input: &str) -> Option<Command> {
	let input = input.trim();
	let (name, rest) = input.split_once(" ").unwrap_or((input, ""));
	let args: Vec<&str> = rest.split_whitespace().collect();

	match (name.to_lowercase().as_str(), args.as_slice()) {
		("newuser", [username, password]) => Some(Command::NewUser(username.to_string(), password.to_string())),
		("login", [username, password]) => Some(Command::Login(username.to_string(), password.to_string())),
		("send", ["all", msg @ ..]) => Some(Command::Send(SendTarget::All, msg.join(" ").trim().to_string())),
		("send", [username, msg @ ..]) => Some(Command::Send(SendTarget::User(username.to_string()), msg.join(" ").trim().to_string())),
		("logout", []) => Some(Command::Logout),
		_ => None,
	}
}