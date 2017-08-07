extern crate msgpack_rpc;
extern crate futures;

use msgpack_rpc::{Client, Handler, Value};
use msgpack_rpc::io::run_tcp;
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::{Ordering, AtomicUsize};
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


#[derive(Default)]
struct Hello {
    counter: AtomicUsize,
}

impl Hello {
    fn value(&self) -> u32 {
        self.counter.load(Ordering::Relaxed) as u32
    }

    fn increment(&self) {
        self.counter.fetch_add(1, Ordering::Relaxed);
    }

    fn into_registry(self) -> Registry {
        let hello = Arc::new(self);

        let mut registry = Registry::default();

        registry.register("value", {
            let hello = hello.clone();
            move |_, _| -> Result<Value, Value> {
                eprintln!("[debug] value()");
                Ok(hello.value().into())
            }
        });

        registry.register_notification("increment", {
            let hello = hello.clone();
            move |_, _| {
                eprintln!("[debug] increment()");
                hello.increment()
            }
        });

        registry
    }
}

fn main() {
    let hello = Hello::default();
    run_tcp(hello.into_registry(), "0.0.0.0:12345")
}
