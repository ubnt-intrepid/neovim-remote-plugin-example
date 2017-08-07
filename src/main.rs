#![allow(dead_code)]

extern crate msgpack_rpc;
extern crate futures;
extern crate rmpv;

use msgpack_rpc::{Client, Handler, Value};
use msgpack_rpc::io::run_stdio;
use std::collections::HashMap;
use futures::{Future, IntoFuture};
use futures::future::{BoxFuture, ok, err};


#[derive(Default)]
struct Registry {
    req: HashMap<String, Box<Fn(Value, &Client) -> Result<Value, Value>>>,
    not: HashMap<String, Box<Fn(Value, &Client)>>,
}

impl Registry {
    fn register<F>(&mut self, method: &str, f: F)
    where
        F: Fn(Value, &Client) -> Result<Value, Value> + 'static,
    {
        self.req.insert(method.to_owned(), Box::new(f));
    }

    fn register_notification<F>(&mut self, method: &str, f: F)
    where
        F: Fn(Value, &Client) + 'static,
    {
        self.not.insert(method.to_owned(), Box::new(f));
    }
}

impl Handler for Registry {
    type RequestFuture = BoxFuture<Value, Value>;
    type NotifyFuture = BoxFuture<(), ()>;

    fn handle_request(&self, method: &str, params: Value, client: &Client) -> Self::RequestFuture {
        match self.req.get(method) {
            Some(ref f) => f(params, client).into_future().boxed(),
            None => err(format!("The function is not found: {:?}", method).into()).boxed(),
        }
    }

    fn handle_notification(
        &self,
        method: &str,
        params: Value,
        client: &Client,
    ) -> Self::NotifyFuture {
        match self.not.get(method) {
            Some(ref f) => {
                f(params, client);
                ok(()).boxed()
            }
            None => err(()).boxed(),
        }
    }
}



fn main() {
    let mut registry = Registry::default();
    registry.register("0:function:Hello", {
        move |params, _| -> Result<Value, Value> {
            let args: Vec<String> = rmpv::ext::from_value(params.as_array().unwrap()[0].clone())
                .map_err(|e| Value::from(e.to_string()))?;
            match args.len() {
                0 => Ok("Who are you?".into()),
                1 => Ok(format!("Hello, {}", args[0]).into()),
                _ => Ok(
                    format!(
                        "Hello, {} and {}",
                        (&args[0..(args.len() - 1)]).join(", "),
                        args[args.len() - 1]
                    ).into(),
                ),
            }
        }
    });

    run_stdio(registry, 4)
}
