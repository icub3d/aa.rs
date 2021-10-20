# aa - API Automation for the Terminal

## Testing With Sample Config

In one terminal:

```bash
npm install -g json-server
json-server test.json
```

This will start up a simple json server that can be used to run the
`sample-config` against. Then you can try running one of the
requests:

```sh
cargo run -- -p sample-config -e local req run json/posts/get
```

```go
package main
func main() {
	fmt.Println("Welcome to json-server!\n")
}
```
