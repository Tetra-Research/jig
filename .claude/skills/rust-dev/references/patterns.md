# Rust Design Patterns Reference

Extended pattern reference for deep dives. Load when exploring specific patterns in detail.

## Creational Patterns

### Builder (Detailed)

The builder pattern separates object construction from representation. In Rust, it's particularly useful because:
- Rust lacks default/optional parameters
- Compile-time validation of required fields is possible
- Method chaining provides clean ergonomics

**Basic Builder:**
```rust
#[derive(Default)]
pub struct ServerBuilder {
    port: Option<u16>,
    host: Option<String>,
    threads: Option<usize>,
}

impl ServerBuilder {
    pub fn port(mut self, port: u16) -> Self {
        self.port = Some(port);
        self
    }

    pub fn host(mut self, host: impl Into<String>) -> Self {
        self.host = Some(host.into());
        self
    }

    pub fn threads(mut self, threads: usize) -> Self {
        self.threads = Some(threads);
        self
    }

    pub fn build(self) -> Result<Server, BuildError> {
        Ok(Server {
            port: self.port.ok_or(BuildError::MissingPort)?,
            host: self.host.unwrap_or_else(|| "localhost".to_string()),
            threads: self.threads.unwrap_or(num_cpus::get()),
        })
    }
}
```

**Typestate Builder (compile-time required field validation):**
```rust
pub struct ServerBuilder<Port, Host> {
    port: Port,
    host: Host,
    threads: Option<usize>,
}

pub struct NoPort;
pub struct HasPort(u16);
pub struct NoHost;
pub struct HasHost(String);

impl ServerBuilder<NoPort, NoHost> {
    pub fn new() -> Self {
        ServerBuilder {
            port: NoPort,
            host: NoHost,
            threads: None,
        }
    }
}

impl<H> ServerBuilder<NoPort, H> {
    pub fn port(self, port: u16) -> ServerBuilder<HasPort, H> {
        ServerBuilder {
            port: HasPort(port),
            host: self.host,
            threads: self.threads,
        }
    }
}

impl<P> ServerBuilder<P, NoHost> {
    pub fn host(self, host: impl Into<String>) -> ServerBuilder<P, HasHost> {
        ServerBuilder {
            port: self.port,
            host: HasHost(host.into()),
            threads: self.threads,
        }
    }
}

// Only available when both required fields are set
impl ServerBuilder<HasPort, HasHost> {
    pub fn build(self) -> Server {
        Server {
            port: self.port.0,
            host: self.host.0,
            threads: self.threads.unwrap_or(num_cpus::get()),
        }
    }
}
```

### Factory

Less common in Rust than OOP languages, but useful for trait object creation:

```rust
pub trait Transport: Send + Sync {
    fn send(&self, data: &[u8]) -> Result<()>;
}

pub fn create_transport(uri: &str) -> Box<dyn Transport> {
    if uri.starts_with("tcp://") {
        Box::new(TcpTransport::new(uri))
    } else if uri.starts_with("unix://") {
        Box::new(UnixTransport::new(uri))
    } else {
        Box::new(MemoryTransport::new())
    }
}
```

## Structural Patterns

### Newtype (Detailed)

Beyond type safety, newtypes enable:
- Custom trait implementations
- Encapsulation of invariants
- Zero-cost abstractions

```rust
/// A validated email address
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Email(String);

impl Email {
    pub fn new(s: impl Into<String>) -> Result<Self, EmailError> {
        let s = s.into();
        if s.contains('@') && s.len() > 3 {
            Ok(Email(s))
        } else {
            Err(EmailError::Invalid(s))
        }
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn domain(&self) -> &str {
        self.0.split('@').nth(1).unwrap()
    }
}

// Derive common traits via deref or explicit impls
impl std::fmt::Display for Email {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}
```

### Adapter

Convert between incompatible interfaces:

```rust
// External crate's interface
pub trait ExternalLogger {
    fn log_message(&self, level: i32, msg: &str);
}

// Your crate's interface
pub trait Logger {
    fn debug(&self, msg: &str);
    fn info(&self, msg: &str);
    fn error(&self, msg: &str);
}

// Adapter
pub struct LoggerAdapter<L: ExternalLogger>(L);

impl<L: ExternalLogger> Logger for LoggerAdapter<L> {
    fn debug(&self, msg: &str) { self.0.log_message(0, msg); }
    fn info(&self, msg: &str) { self.0.log_message(1, msg); }
    fn error(&self, msg: &str) { self.0.log_message(2, msg); }
}
```

### Decorator

Add behavior without modifying the original:

```rust
pub trait DataSource {
    fn read(&self) -> Vec<u8>;
    fn write(&mut self, data: &[u8]);
}

pub struct EncryptedDataSource<D: DataSource> {
    inner: D,
    key: [u8; 32],
}

impl<D: DataSource> DataSource for EncryptedDataSource<D> {
    fn read(&self) -> Vec<u8> {
        let encrypted = self.inner.read();
        decrypt(&encrypted, &self.key)
    }

    fn write(&mut self, data: &[u8]) {
        let encrypted = encrypt(data, &self.key);
        self.inner.write(&encrypted)
    }
}
```

## Behavioral Patterns

### Strategy

Encapsulate algorithms as traits:

```rust
pub trait CompressionStrategy {
    fn compress(&self, data: &[u8]) -> Vec<u8>;
    fn decompress(&self, data: &[u8]) -> Vec<u8>;
}

pub struct GzipStrategy;
pub struct LzmaStrategy;
pub struct NoCompression;

impl CompressionStrategy for GzipStrategy { /* ... */ }
impl CompressionStrategy for LzmaStrategy { /* ... */ }
impl CompressionStrategy for NoCompression { /* ... */ }

pub struct FileProcessor<C: CompressionStrategy> {
    compression: C,
}
```

### Command

Encapsulate operations:

```rust
pub trait Command {
    fn execute(&mut self);
    fn undo(&mut self);
}

pub struct InsertText {
    buffer: Rc<RefCell<String>>,
    position: usize,
    text: String,
}

impl Command for InsertText {
    fn execute(&mut self) {
        self.buffer.borrow_mut().insert_str(self.position, &self.text);
    }

    fn undo(&mut self) {
        let mut buf = self.buffer.borrow_mut();
        buf.drain(self.position..self.position + self.text.len());
    }
}
```

### State (Typestate)

Encode state machines with compile-time guarantees:

```rust
// States
pub struct Draft;
pub struct PendingReview;
pub struct Published;

pub struct Post<S> {
    content: String,
    _state: PhantomData<S>,
}

impl Post<Draft> {
    pub fn new(content: String) -> Self {
        Post { content, _state: PhantomData }
    }

    pub fn request_review(self) -> Post<PendingReview> {
        Post { content: self.content, _state: PhantomData }
    }
}

impl Post<PendingReview> {
    pub fn approve(self) -> Post<Published> {
        Post { content: self.content, _state: PhantomData }
    }

    pub fn reject(self) -> Post<Draft> {
        Post { content: self.content, _state: PhantomData }
    }
}

impl Post<Published> {
    pub fn content(&self) -> &str {
        &self.content
    }
}
```

## Rust-Specific Patterns

### Interior Mutability

When you need mutation through shared references:

```rust
use std::cell::{Cell, RefCell};

// Cell - for Copy types
pub struct Counter {
    count: Cell<u32>,
}

impl Counter {
    pub fn increment(&self) {
        self.count.set(self.count.get() + 1);
    }
}

// RefCell - for non-Copy types, runtime borrow checking
pub struct Cache {
    data: RefCell<HashMap<String, String>>,
}

impl Cache {
    pub fn get_or_insert(&self, key: &str, f: impl FnOnce() -> String) -> String {
        if let Some(v) = self.data.borrow().get(key) {
            return v.clone();
        }
        let value = f();
        self.data.borrow_mut().insert(key.to_string(), value.clone());
        value
    }
}
```

### RAII Guards

Automatic cleanup via Drop:

```rust
pub struct TempFile {
    path: PathBuf,
}

impl TempFile {
    pub fn new() -> io::Result<Self> {
        let path = std::env::temp_dir().join(uuid::Uuid::new_v4().to_string());
        File::create(&path)?;
        Ok(TempFile { path })
    }

    pub fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for TempFile {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.path);
    }
}
```

### Extension Traits

Add methods to types you don't own:

```rust
pub trait ResultExt<T, E> {
    fn log_err(self) -> Option<T>;
}

impl<T, E: std::fmt::Debug> ResultExt<T, E> for Result<T, E> {
    fn log_err(self) -> Option<T> {
        match self {
            Ok(v) => Some(v),
            Err(e) => {
                eprintln!("Error: {:?}", e);
                None
            }
        }
    }
}
```

### Blanket Implementations

Implement traits for all types matching a bound:

```rust
pub trait Printable {
    fn print(&self);
}

// Implement for all Display types
impl<T: std::fmt::Display> Printable for T {
    fn print(&self) {
        println!("{}", self);
    }
}
```
