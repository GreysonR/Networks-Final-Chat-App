/**
 * October 6 2026
 * Greyson Rockwell
 * 
 * Handles "DB" logic: Data structures for the users, parsing + writing to the users.txt file
 */


use std::{fs, collections::HashMap, sync::{Arc, Mutex}};
use tokio::net::tcp::OwnedWriteHalf;

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
	pub fn get(&self, username: &str) -> Option<Arc<User>> {
		self.users.lock().unwrap()
			.get(username)
			.map(|value| { Arc::clone(value) })
	}

	pub fn contains(&self, username: &str) -> bool {
		self.users.lock().unwrap()
			.contains_key(username)
	}
	// pub fn is_online(&self, username: &str) -> bool {}

	pub fn insert(&self, username: &str, password: &str, tcp_write: Option<OwnedWriteHalf>) {
		let tcp_write = tcp_write.map(|stream| Arc::new(Mutex::new(stream)));
		self.users.lock().unwrap()
			.insert(username.to_string(), Arc::new(User::new(username, password, tcp_write)));
		self.save();
	}

	pub async fn iter_keys(&self, method: impl AsyncFn(&String) -> ()) {
		for key in self.users.lock().unwrap().keys() {
			method(key).await;
		}
	}

	fn save(&self) {
		let mut contents = String::new();
		let users = self.users.lock().unwrap();
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
