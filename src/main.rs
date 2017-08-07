extern crate msgpack_rpc;
extern crate futures;
extern crate tokio_core;

use msgpack_rpc::{Client, Endpoint, Handler, Value};
use msgpack_rpc::io::StdioStream;
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::{Ordering, AtomicUsize};
use futures::{Future, Stream, IntoFuture};
use futures::future::{BoxFuture, empty, ok, err};
use tokio_core::reactor::Core;
use tokio_core::net::TcpListener;


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


#[allow(dead_code)]
fn run(registry: Registry) {
    let mut core = Core::new().unwrap();
    let handle = core.handle();

    let io = StdioStream::new(4);
    let endpoint = Endpoint::from_io(&handle, io);

    endpoint.serve(&handle, registry);
    core.run(empty::<(), ()>()).unwrap();
}

fn run_tcp(registry: Registry, addr: &str) {
    let registry = Arc::new(registry);

    let mut core = Core::new().unwrap();
    let handle = core.handle();

    let addr = addr.parse().unwrap();
    let listener = TcpListener::bind(&addr, &handle).unwrap();

    let server = listener.incoming().for_each(move |(sock, _addr)| {
        let endpoint = Endpoint::from_io(&handle, sock);
        endpoint.serve(&handle, registry.clone());
        Ok(())
    });

    core.run(server).unwrap();
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
