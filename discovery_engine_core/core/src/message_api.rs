// Copyright 2021 Xayn AG
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as
// published by the Free Software Foundation, version 3.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.

//! Sketch of the message passing API
//!
//! The idea is to structure it following way:
//!
//! - in `discovery_engine_core` we provides a _rust_ message passing API which
//!   will be our public engine API in the future
//!     - i.e. the implementation of System
//! - we will then provide language specific bindings for languages we need to
//!   bind to, for now only dart
//!     - i.e. the implementation of `RecvRequest`, `SendResponse` and a bit of
//!       ffi glue code, e.g. for dart exactly 2 ffi functions (for allo_isolate,
//!       1 for dart-api-dl)
//! - the message passing will send byte messages, the en-/de-coding of them
//!   is done separately from this interface and their FFI bindings
//!     - depending on the FFI bindings we either send a package consisting
//!       of a bytes-message and a magic cookie, or we append the magic cookie
//!       to the byte message
//!
//! In the future we can split this module and language bindings non specific
//! to this module out of this repo as a set of libraries if we thing it's helpful.
//!
//! Normally we would make `Engine` implement `System`, but for the beginning this
//! has to exist in parallel with the previous FFI API. As such we have to to temp.
//! do some things differently:
//!
//! 1. implement System in the binding crate, instead of the core crate where it is
//!    supposed to be implemented
//! 2. do some hacky thing to make sure both the FFI and message API have access to
//!    the engine

use std::{
    any::Any,
    sync::{
        atomic::{self, AtomicI64},
        Arc,
    },
};

use async_trait::async_trait;
use thiserror::Error;

/// Initialize the system, setting up message passing.
///
/// # Panic
///
/// - This panics if it is not called in the context of a tokio
///   runtime, the message handling loop will run in the given
///   tokio runtime.
pub fn init<S, F, T>(init_message: Package, send_response: F, recv_request: T)
where
    S: System<F>,
    F: SendResponse,
    T: RecvRequest,
{
    let send_last_response = send_response.clone();

    // We have to spawn two futures, to work around limitations of `catch_unwind`.
    let main_task_handle = tokio::spawn(async move {
        let cookie = init_message.cookie;
        let mut system = match S::create(init_message, send_response.clone()).await {
            Ok(system) => system,
            Err(err) => {
                return Some(Package {
                    cookie,
                    message: err,
                });
            }
        };

        while let Some(msg) = recv_request.recv().await {
            let cookie = msg.cookie;
            let message = system
                .handle_message(msg)
                .await
                .unwrap_or_else(|err_msg| err_msg);

            if let Err(ResponseChannelBroken) =
                send_response.send(Package { message, cookie }).await
            {
                return None;
            }
        }

        let message = system.shutdown().await;
        Some(Package {
            message,
            cookie: MagicCookie::generate_cookie(),
        })
    });

    tokio::spawn(async move {
        let res = main_task_handle.await.unwrap_or_else(|join_error| {
            let panic_payload = join_error.try_into_panic().ok();
            Some(Package {
                message: S::create_future_aborted_error(panic_payload),
                cookie: MagicCookie::generate_cookie(),
            })
        });

        if let Some(msg) = res {
            send_last_response.send(msg).await.ok();
        }
    });
}

/// Magic cookie used to link requests and responses.
///
/// As we can generate responses without requests we need to
/// have a way to generate id's of both sides of the bindings without
/// conflict. The solution is that id's generate in rust are all negative
/// while ids generated outside are all positive. By having 63bits for
/// each id generator we can be sure it practically can't overflow. Note
/// that we can't enforce this as we might want to respond to out-of-band
/// response send by the engine.
#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
#[repr(transparent)]
pub struct MagicCookie(pub i64);

impl MagicCookie {
    /// Generates a cookie for a response without a request.
    pub fn generate_cookie() -> Self {
        static COOKIE_GEN: AtomicI64 = AtomicI64::new(-1);
        MagicCookie(COOKIE_GEN.fetch_sub(1, atomic::Ordering::AcqRel))
    }
}

pub struct Message {
    pub message: Vec<u8>,
}

pub struct Package {
    pub message: Message,
    pub cookie: MagicCookie,
}

#[derive(Debug, Error)]
#[error("the response channel is permanently broken")]
pub struct ResponseChannelBroken;

#[async_trait]
pub trait System<SR>: Send + 'static + Sized
where
    SR: SendResponse,
{
    /// Create the system using the initial message;
    ///
    /// A clone of the `SendResponse` instance is passed in so that
    /// we can:
    ///
    /// 1. generate multiple responses for the same request
    /// 2. generate responses not associated with a request, e.g.
    ///    generate by a background job
    async fn create(init_message: Package, send_response: SR) -> Result<Self, Message>;

    /// Handle the message, we expect a response message we can send back.
    ///
    /// Be aware that while this returns a result this is just for convenience of
    /// being able to internally use a `Error` type which implements `Into<Message>`.
    /// But both the `Ok` and `Err` messages are handled the same.
    async fn handle_message(&mut self, msg: Package) -> Result<Message, Message>;

    /// Shutdown the system after knowing that we won't receive any more messages.
    ///
    /// This mainly should generate some "shutdown signaling" message.
    ///
    /// The is currently no way to shutdown a system from within, besides
    /// panicing. We maybe might want to change this in the future.
    async fn shutdown(self) -> Message;

    /// Create a error message in case the future was aborted.
    ///
    /// Should not panic.
    ///
    /// Is separate from logging, as such using the payload is not necessary
    /// required, we might just create some static "well known" protobuf message
    /// or similar in the future.
    ///
    /// If `None` is passed in the future was canceled, if `Some` was passed in
    /// it died of a panic.
    fn create_future_aborted_error(panic_payload: Option<Box<dyn Any + Send>>) -> Message;
}

#[async_trait]
pub trait SendResponse: Send + Sync + Clone + 'static {
    async fn send(&self, msg: Package) -> Result<(), ResponseChannelBroken>;
}

/// Allow sending responses to a tokio channel, useful for integration tests.
#[async_trait]
impl SendResponse for tokio::sync::mpsc::Sender<Package> {
    async fn send(&self, msg: Package) -> Result<(), ResponseChannelBroken> {
        self.send(msg).await.map_err(|_| ResponseChannelBroken)
    }
}

/// Allows sending responses using a non-async closure, useful for enqueuing
/// messages in a dart `SendPort` and similar.
#[async_trait]
impl SendResponse for Arc<dyn Fn(Package) -> Result<(), ResponseChannelBroken> + Send + Sync> {
    async fn send(&self, msg: Package) -> Result<(), ResponseChannelBroken> {
        self(msg)
    }
}

#[async_trait]
pub trait RecvRequest: Send + Sync + 'static {
    async fn recv(&self) -> Option<Package>;
}

#[async_trait]
impl RecvRequest for tokio::sync::mpsc::Receiver<Package> {
    async fn recv(&self) -> Option<Package> {
        self.recv().await
    }
}
