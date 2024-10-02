
# Contributing to Refact Agent

In short:

* Fork the repository
* Create a feature branch
* Do the work
* Create a pull request
* Maintainers will review it


## What Can I Do?

Before you start, create an issue with a title that begins with `[idea] ...`. The field of AI and agents is vast,
and not every idea will benefit the project, even if it is a good idea in itself.

Another rule of thumb: Only implement a feature you can test thoroughly.


## Coding Standards

Good practices for Rust are generally applicable to this project. There are a few points however:

1. Naming. Use "Find in files..." to check if a name you give to your structs, fields, functions is too
generic. If a name is already all over the project, be more specific. For example "IntegrationGitHub" is a good
name, but "Integration" is not, even if it's in `github.rs` and files work as namespaces in Rust. It's
still hard to navigate the project if you can't use search.

2. Locks. For some reason, it's still hard for most people, and for current AI models, too. Refact is
multi-threaded, locks are necessary. But locks need to be activated for the shortest time possible, this
is how you use `Arc<AMutex<>>` to do it:

```rust
struct BigStruct {
    ...
    pub small_struct: Arc<AMutex<SmallStruct>>;
}

fn some_code(big_struct: Arc<AMutex<BigStruct>>)
{
    let small_struct = {
        let big_struct_locked = big_struct.lock().await;
        big_struct_locked.small_struct.clone()  // cloning Arc is cheap
        // big_struct_locked is destroyed here because it goes out of scope
    };
    // use small_struct without holding big_struct_locked
}
```

Another multi-threaded trick, move a member function outside of a class:

```rust
struct MyStruct {
    pub data1: i32,
    pub data2: i32,
}

impl MyStruct {
    pub fn lengthy_function1(&mut self)  {  }
}

fn some_code(my_struct: Arc<AMutex<SmallStruct>>)
{
    my_struct.lock().await.lengthy_function1();
    // Whoops, lengthy_function has the whole structure locked for a long time,
    // and Rust won't not let you unlock it
}

pub fn lengthy_function2(s: Arc<AMutex<SmallStruct>>)
{
    let (data1, data2) = {
        let s_locked = s.lock().await;
        (s_locked.data1.clone(), s_locked.data2.clone())
    }
    // Do lengthy stuff here without locks!
}
```


## Testing

It's a good idea to have tests in source files, and run them using `cargo test`, and we
have CI in place to run it automatically.
But not everything can be tested solely within Rust tests, for example a Rust test cannot run
an AI model inside.

So we have `tests/*.py` scripts that expect the `refact-lsp` process to be running on port 8001,
and the project itself as a workspace dir:


```bash
cargo build && target/debug/refact-lsp --http-port 8001 --reset-memory --experimental --workspace-folder . --logs-stderr --vecdb --ast
```

Running those tests is still manual. To make sure your work didn't break other features,
run tests for things you might have broken.


## Contact

If you have any questions or concerns, please contact the project maintainers on Discord:
https://www.smallcloud.ai/discord

