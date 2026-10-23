/**
 * October 23 2026
 * Greyson Rockwell
 * 
 * Parses user input into Commands that can be processed by the program
 */

#[derive(Debug)]
pub enum SendTarget {
	All,
	User(String),
}
// impl Display so you can print the command into a string and it just works.
impl std::fmt::Display for SendTarget {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			SendTarget::All => write!(f, "all"),
			SendTarget::User(username) => write!(f, "{username}"),
		}
	}
}

#[derive(Debug)]
pub enum Command {
	NewUser(String, String),
	Login(String, String),
	Send(SendTarget, String),
	Who,
	Logout,
}

pub fn parse_command(input: &str) -> Option<Command> {
	let input = input.trim();
	if input == "send" { return Some(Command::Send(SendTarget::User(String::from(".")), String::new())); } // hacky way to get example working. imo the client should just respond with "invalid input" when there's no parameters for the send command (which is what the program does by default), but the assignment requires responding with "please log in." 
	let (name, rest) = input.split_once(" ").unwrap_or((input, ""));
	let args: Vec<&str> = rest.split_whitespace().collect();

	match (name.to_lowercase().as_str(), args.as_slice()) {
		("newuser", [username, password]) => Some(Command::NewUser(username.to_string(), password.to_string())),
		("login", [username, password]) => Some(Command::Login(username.to_string(), password.to_string())),
		("send", _) => {
			let (target, msg) = rest.trim_start().split_once(" ")?;
			let msg = msg.trim();
			if msg.is_empty() { return None; }

			let target = if target.eq_ignore_ascii_case("all") { SendTarget::All } else { SendTarget::User(target.to_string()) };
			Some(Command::Send(target, msg.to_string()))
		},
		("who", []) => Some(Command::Who),
		("logout", []) => Some(Command::Logout),
		_ => None,
	}
}