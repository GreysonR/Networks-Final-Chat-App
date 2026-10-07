#[derive(Debug)]
pub enum Command {
	NewUser(String, String),
	Login(String, String),
	Send(String),
	Logout,
}

pub fn parse_command(input: &str) -> Option<Command> {
	let input = input.trim();
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