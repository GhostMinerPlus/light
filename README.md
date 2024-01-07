# Light
A web server

# Quick start
```sh
light [config.toml] --port 8005
```
config.toml
```toml
# name = "light"
# ip = "0.0.0.0"
# port = 80
# path = "/light"
# hosts = []
# proxy = {}
# log_level = "INFO"
# src = "dist"
# thread_num = 8
```
Then it will serving at http://$ip:$port/$name

# Freature
- Dynamic proxy: use middleware, add, remove, list
