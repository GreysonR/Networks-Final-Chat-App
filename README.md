# Networks Final Project
This is the repo for my submission of the final project in CS4850 Computer Networks. It implements a simple CLI chat app with a server and client using sockets. 

# Requirements
- [rust](https://rustup.rs/)

# Running
The server uses the port `18747` by default (per assignment spec), which cannot be configured outside of editing the code. Please ensure that port is open or it will fail.
- Start the server: `cargo run -p server`
- Run a client instance: `cargo run -p client -- 127.0.0.1`

# Client Commands
- `newuser [username] [password]`: Creates a new username with the given username and password.
- `login [username] [password]`: Logs into an existing user
- `send [message]`: Sends the message to the server
- `logout`: Closes the connection & logs out
