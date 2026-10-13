//! A browser WebSocket that reconnects until it is dropped.

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use wasm_bindgen::JsCast;
use wasm_bindgen::closure::Closure;
use web_sys::{MessageEvent, WebSocket};

/// What the owner hears from the socket.
pub trait Listener {
    fn on_text(&self, text: &str);
    /// The socket closed before it delivered a message: the host refused
    /// or could not be reached.
    fn on_refused(&self);
}

/// The longest wait between reconnect attempts.
const MAX_RETRY_MS: i32 = 15_000;

/// A JavaScript callback, set once the socket exists to point back at it.
type Callback<A> = RefCell<Option<Closure<dyn FnMut(A)>>>;

struct Inner {
    url: String,
    listener: Box<dyn Listener>,
    socket: RefCell<Option<WebSocket>>,
    on_message: Callback<MessageEvent>,
    on_close: RefCell<Option<Closure<dyn FnMut()>>>,
    retry: RefCell<Option<Closure<dyn FnMut()>>>,
    /// The pending reconnect, cleared on drop so it never calls a freed
    /// closure.
    timer: Cell<Option<i32>>,
    received: Cell<bool>,
    retry_ms: Cell<i32>,
}

/// An open stream. Dropping it closes the connection.
pub struct Socket {
    inner: Rc<Inner>,
}

impl Socket {
    pub fn open(url: String, listener: impl Listener + 'static) -> Self {
        let inner = Rc::new(Inner {
            url,
            listener: Box::new(listener),
            socket: RefCell::new(None),
            on_message: RefCell::new(None),
            on_close: RefCell::new(None),
            retry: RefCell::new(None),
            timer: Cell::new(None),
            received: Cell::new(false),
            retry_ms: Cell::new(1_000),
        });
        let weak = Rc::downgrade(&inner);
        *inner.on_message.borrow_mut() = Some(Closure::new({
            let weak = weak.clone();
            move |event: MessageEvent| {
                let (Some(inner), Some(text)) = (weak.upgrade(), event.data().as_string()) else {
                    return;
                };
                inner.received.set(true);
                inner.retry_ms.set(1_000);
                inner.listener.on_text(&text);
            }
        }));
        *inner.on_close.borrow_mut() = Some(Closure::new({
            let weak = weak.clone();
            move || {
                if let Some(inner) = weak.upgrade() {
                    inner.closed();
                }
            }
        }));
        *inner.retry.borrow_mut() = Some(Closure::new(move || {
            if let Some(inner) = weak.upgrade() {
                inner.timer.set(None);
                inner.connect();
            }
        }));
        inner.connect();
        Self { inner }
    }
}

impl Inner {
    fn connect(&self) {
        let Ok(socket) = WebSocket::new(&self.url) else {
            self.listener.on_refused();
            return;
        };
        if let Some(handler) = self.on_message.borrow().as_ref() {
            socket.set_onmessage(Some(handler.as_ref().unchecked_ref()));
        }
        if let Some(handler) = self.on_close.borrow().as_ref() {
            socket.set_onclose(Some(handler.as_ref().unchecked_ref()));
        }
        *self.socket.borrow_mut() = Some(socket);
    }

    /// Reconnects after a wait that doubles with each failure, or reports a
    /// host that never answered.
    fn closed(&self) {
        if !self.received.get() {
            self.listener.on_refused();
        }
        let delay = self.retry_ms.get();
        self.retry_ms.set((delay * 2).min(MAX_RETRY_MS));
        let retry = self.retry.borrow();
        let (Some(window), Some(retry)) = (web_sys::window(), retry.as_ref()) else {
            return;
        };
        match window.set_timeout_with_callback_and_timeout_and_arguments_0(
            retry.as_ref().unchecked_ref(),
            delay,
        ) {
            Ok(timer) => self.timer.set(Some(timer)),
            Err(error) => log::warn!("cannot schedule a reconnect: {error:?}"),
        }
    }
}

impl Drop for Socket {
    fn drop(&mut self) {
        if let (Some(timer), Some(window)) = (self.inner.timer.take(), web_sys::window()) {
            window.clear_timeout_with_handle(timer);
        }
        if let Some(socket) = self.inner.socket.borrow_mut().take() {
            socket.set_onmessage(None);
            socket.set_onclose(None);
            let _ = socket.close();
        }
    }
}
