# Turnip Http
A simple to use http library aimed towards users who want to finish their projects.

## Getting Started
At the core of turnip http is a very simple set of handler systems and routing modules. Let's start with a basic example and break it down.

```rs
    fn hello_world(request: Request) -> u32 {
        println!("Received hello world request");

        200
    }

    fn main() {
        let router = Router::new().insert("");

        let frame = Framework {...}
    }
```
