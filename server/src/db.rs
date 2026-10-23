/**
 * October 6 2026
 * Greyson Rockwell
 * 
 * Handles "DB" logic: Data structures for the users, parsing + writing to the users.txt file
 */


use std::{fs, collections::HashMap, sync::Arc, io::{Error, ErrorKind}};
use tokio::{io::AsyncWriteExt, net::tcp::OwnedWriteHalf, sync::Mutex};

pub struct User {
	pub username: String,
	pub password: String,

	pub socket_write: Option<Arc<Mutex<OwnedWriteHalf>>>,
}
impl User {
	pub fn new(username: &str, password: &str, socket_write: Option<Arc<Mutex<OwnedWriteHalf>>>) -> Self {
		Self {
			username: username.to_string().trim().to_string(),
			password: password.to_string().trim().to_string(),
			socket_write,
		}
	}
}

pub struct Users {
	filename: String,
	users: Mutex<HashMap<String, Arc<User>>>,
}
impl Users {
	fn parse_file(filename: &str) -> Self {
		let contents = match fs::read_to_string(filename) {
			Ok(data) => data,
			Err(_) => String::new(), // Assume file doesn't exist; file will be created on save
		};
		let mut users = HashMap::new();
		let contents = contents
			.lines()
			.map(|line| {
				line
					.strip_circumfix("(", ")").expect("Invalid format for users file")
					.split_once(",").expect("Invalid format for users file")
			});

		for user_data in contents {
			let (username, password) = user_data;

			let user = User::new(username, password, None);
			users.insert(username.to_string(), Arc::new(user));
		}

		Self {
			filename: filename.to_string(),
			users: Mutex::new(users)
		}
	}
	pub fn new(filename: &str) -> Self {
		Self::parse_file(filename)
	}
	pub async fn get(&self, username: &str) -> Option<Arc<User>> {
		self.users.lock().await
			.get(username)
			.map(|value| { Arc::clone(value) })
	}

	pub async fn contains(&self, username: &str) -> bool {
		self.users.lock().await
			.contains_key(username)
	}
	// pub fn is_online(&self, username: &str) -> bool {}

	pub async fn insert(&self, username: &str, password: &str, tcp_write: Option<OwnedWriteHalf>) {
		let tcp_write = tcp_write.map(|stream| Arc::new(Mutex::new(stream)));
		self.users.lock().await
			.insert(username.to_string(), Arc::new(User::new(username, password, tcp_write)));
		self.save().await;
	}

	// Sends message `msg` to the user. Errors if the user is unknown or offline.
	pub async fn send(&self, to: &str, msg: &str) -> std::io::Result<()> {
		let user = self.get(to).await
			.ok_or_else(|| Error::new(ErrorKind::NotFound, "no such user"))?;
		let write = user.socket_write.clone()
			.ok_or_else(|| Error::new(ErrorKind::NotConnected, "user is offline"))?;

		let mut write = write.lock().await;
		write.write_all(msg.as_bytes()).await
	}

	// Writes raw `msg` to every user. Users with errors (i.e. offline) are skipped.
	pub async fn broadcast(&self, msg: &str) {
		let names: Vec<String> = self.users.lock().await.keys().cloned().collect();
		for name in names {
			let _ = self.send(&name, msg).await;
		}
	}

	async fn save(&self) {
		let mut contents = String::new();
		let users = self.users.lock().await;
		for user in users.values() {
			let username = &user.username;
			let password = &user.password;

			contents += &format!("({username}, {password})\n");
		}

		let _ = fs::write(&self.filename, contents).map_err(|err| {
			eprintln!("Failed to write to file {}: {err}", &self.filename);
		});
	}
}
