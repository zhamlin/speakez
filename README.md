# SpeakEZ
A mumble client and server written in Rust.

## Overview
This repo is proof of concept and is not intended to be used in place of the original mumble server at this time.

### Features
- Supports existing mumble clients
- Web client
- Mobile support via [Tauri](https://github.com/tauri-apps/tauri)

### Running
```
# Start the mumble server
make run-server

# Start the web server for the web client.
# Runs on localhost:8080
make run-web
```

# Inspiration
- https://github.com/mumble-voip/mumble
- https://github.com/Johni0702/mumble-web
