# Usage
To start the signaling server:
``` sh
cd matchmaker
cargo run
```

To add users: 
``` sh
curl -X POST -H "Content-type: application/json" -d '{"username": "alice", "pwd": "secret"}' http://127.0.0.1:3657/user/add -v
```
