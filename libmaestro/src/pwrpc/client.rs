use std::pin::Pin;
use std::task::Poll;

use futures::{Sink, SinkExt, Stream, StreamExt};
use futures::channel::mpsc;
use futures::stream::{SplitSink, SplitStream, FusedStream};

use prost::Message;

use super::id::Path;
use super::status::{Status, Error};
use super::types::{RpcType, RpcPacket, PacketType};


#[derive(Debug)]
pub struct Client<S> {
    /// Stream for lower-level transport.
    io_rx: SplitStream<S>,

    /// Sink for lower-level transport.
    io_tx: SplitSink<S, RpcPacket>,

    /// Queue receiver for requests to be processed and sent by us.
    queue_rx: mpsc::UnboundedReceiver<CallRequest>,

    /// Queue sender for requests to be processed by us. Counter-part for
    /// `queue_rx`, used by callers via `ClientHandle` to initiate new calls.
    queue_tx: mpsc::UnboundedSender<CallRequest>,

    /// Pending RPC calls, waiting for a response.
    pending: Vec<Call>,
}

impl<S, E> Client<S>
where
    S: Sink<RpcPacket>,
    S: Stream<Item = Result<RpcPacket, E>> + Unpin,
    Error: From<S::Error>,
    Error: From<E>,
{
    pub fn new(stream: S) -> Client<S> {
        let (io_tx, io_rx) = stream.split();
        let (queue_tx, queue_rx) = mpsc::unbounded();

        Client {
            io_rx,
            io_tx,
            queue_rx,
            queue_tx,
            pending: Vec::new(),
        }
    }

    pub fn handle(&self) -> ClientHandle {
        ClientHandle {
            queue_tx: self.queue_tx.clone(),
        }
    }

    pub async fn run(&mut self) -> Result<(), Error> {
        // Process the request queue first in case we are trying to catch some
        // early RPC responses via open() calls.
        while let Ok(Some(request)) = self.queue_rx.try_next() {
            self.process_request(request).await?;
        }

        loop {
            tokio::select! {
                packet = self.io_rx.next() => {
                    let packet = packet
                        .ok_or_else(|| Error::aborted("underlying IO stream closed"))??;

                    self.process_packet(packet).await?;
                },
                request = self.queue_rx.next() => {
                    // SAFETY: We hold both sender and receiver parts and are
                    // the only ones allowed to close this queue. Therefore, it
                    // will always be open here.
                    let request = request.expect("request queue closed unexpectedly");

                    self.process_request(request).await?;
                },
            }
        }
    }

    pub async fn terminate(&mut self) -> Result<(), Error> {
        tracing::trace!("terminating client");

        // Collect messages to be sent instead of directly sending them. We
        // process infallible (local) operations first, before we try to
        // communicate with the RPC peer, which is fallible.
        let mut send = Vec::new();

        // Close our request queue.
        self.queue_rx.close();

        // Process all pending requests. Abort requests for new calls and
        // send/forward any errors.
        //
        // SAFETY: try_next() can only return an error when the channel has not
        // been closed yet.
        while let Some(msg) = self.queue_rx.try_next().unwrap() {
            match msg {
                CallRequest::New { sender, .. } => {
                    // Drop new requests. Instead, notify caller with status 'aborted'.
                    let update = CallUpdate::Error { status: Status::Aborted };
                    let _ = sender.unbounded_send(update);
                    sender.close_channel();
                },
                CallRequest::Error { uid, code, tx } => {
                    // Process error requests as normal: Send error message to
                    // peer, remove and complete call.
                    if let Some(mut call) = self.find_and_remove_call(uid) {
                        call.complete_with_error(code).await;
                        if tx {
                            send.push((uid, code));
                        }
                    }
                },
            }
        }

        // Cancel all pending RPCs and remove them from the list.
        for call in &mut self.pending {
            call.complete_with_error(Status::Aborted).await;
            send.push((call.uid, Status::Cancelled));
        }
        self.pending.clear();

        // Define functions because async try-catch blocks aren't a thing yet...
        async fn do_send<S, E>(client: &mut Client<S>, send: Vec<(CallUid, Status)>) -> Result<(), Error>
        where
            S: Sink<RpcPacket>,
            S: Stream<Item = Result<RpcPacket, E>> + Unpin,
            Error: From<S::Error>,
            Error: From<E>,
        {
            for (uid, code) in send {
                client.send_client_error(uid, code).await?;
            }
            Ok(())
        }

        async fn do_close<S, E>(client: &mut Client<S>) -> Result<(), Error>
        where
            S: Sink<RpcPacket>,
            S: Stream<Item = Result<RpcPacket, E>> + Unpin,
            Error: From<S::Error>,
            Error: From<E>,
        {
            client.io_tx.close().await?;
            Ok(())
        }

        // Try to send cancel/error messages.
        let res_send = do_send(self, send).await;

        // Try to close the transport.
        let res_close = do_close(self).await;

        // Return the first error.
        res_send?;
        res_close
    }

    async fn process_packet(&mut self, packet: RpcPacket) -> Result<(), Error> {
        tracing::trace!(
            "received packet: type=0x{:02x}, channel_id=0x{:02x}, service_id=0x{:08x}, method_id=0x{:08x}, call_id=0x{:02x}",
            packet.r#type, packet.channel_id, packet.service_id, packet.method_id, packet.call_id
        );

        let ty = packet.r#type;
        let ty = PacketType::try_from(ty);

        match ty {
            Ok(PacketType::Response) => {
                self.rpc_complete(packet).await
            },
            Ok(PacketType::ServerError) => {
                self.rpc_complete_with_error(packet).await
            },
            Ok(PacketType::ServerStream) => {
                self.rpc_stream_push(packet).await?
            },
            Ok(_) => {
                tracing::error!(
                    "unsupported packet type: type=0x{:02x}, channel_id=0x{:02x}, service_id=0x{:08x}, method_id=0x{:08x}, call_id=0x{:02x}",
                    packet.r#type, packet.channel_id, packet.service_id, packet.method_id, packet.call_id
                );
            },
            Err(_) => {
                tracing::error!(
                    "unknown packet type: type=0x{:02x}, channel_id=0x{:02x}, service_id=0x{:08x}, method_id=0x{:08x}, call_id=0x{:02x}",
                    packet.r#type, packet.channel_id, packet.service_id, packet.method_id, packet.call_id
                );
            },
        }

        Ok(())
    }

    async fn rpc_complete(&mut self, packet: RpcPacket) {
        let uid = CallUid::from_packet(&packet);
        let call = self.find_and_remove_call(uid);

        match call {
            Some(mut call) => {     // pending call found, complete rpc
                tracing::trace!(
                    "completing rpc: channel_id=0x{:02x}, service_id=0x{:08x}, method_id=0x{:08x}, call_id=0x{:02x}",
                    packet.channel_id, packet.service_id, packet.method_id, packet.call_id
                );

                if packet.status != 0 {
                    tracing::warn!(
                        "completing rpc with non-zero status: channel_id=0x{:02x}, service_id=0x{:08x}, method_id=0x{:08x}, call_id=0x{:02x}, status={}",
                        packet.channel_id, packet.service_id, packet.method_id, packet.call_id, packet.status
                    );
                }

                let status = Status::from(packet.status);
                call.complete(packet.payload, status).await;
            },
            None => {               // no pending call found, silently drop packet
                tracing::debug!(
                    "received response for non-pending rpc: channel_id=0x{:02x}, service_id=0x{:08x}, method_id=0x{:08x}, call_id=0x{:02x}",
                    packet.channel_id, packet.service_id, packet.method_id, packet.call_id
                );
            },
        }
    }

    async fn rpc_complete_with_error(&mut self, packet: RpcPacket) {
        let uid = CallUid::from_packet(&packet);
        let call = self.find_and_remove_call(uid);

        match call {
            Some(mut call) => {     // pending call found, complete rpc with error
                tracing::trace!(
                    "completing rpc with error: channel_id=0x{:02x}, service_id=0x{:08x}, method_id=0x{:08x}, call_id=0x{:02x}, status={}",
                    packet.channel_id, packet.service_id, packet.method_id, packet.call_id, packet.status
                );

                let status = Status::from(packet.status);
                call.complete_with_error(status).await;
            },
            None => {               // no pending call found, silently drop packet
                tracing::debug!(
                    "received error for non-pending rpc: channel_id=0x{:02x}, service_id=0x{:08x}, method_id=0x{:08x}, call_id=0x{:02x}, status={}",
                    packet.channel_id, packet.service_id, packet.method_id, packet.call_id, packet.status
                );
            },
        }
    }

    async fn rpc_stream_push(&mut self, packet: RpcPacket) -> Result<(), Error> {
        let uid = CallUid::from_packet(&packet);
        let call = self.find_call_mut(uid);

        match call {
            Some(call) => {         // pending call found, forward packet to caller
                tracing::trace!(
                    "pushing server stream packet to caller: channel_id=0x{:02x}, service_id=0x{:08x}, method_id=0x{:08x}, call_id=0x{:02x}",
                    packet.channel_id, packet.service_id, packet.method_id, packet.call_id
                );

                if call.ty.has_server_stream() {    // packet was expected, forward it
                    call.push_item(packet.payload).await;
                } else {            // this type of rpc doesn't expect streaming packets from the server
                    // SAFETY: We are the only ones that can add, remove, or
                    //         otherwise modify items in-between the above find
                    //         operation and this one as we have the lock.
                    let mut call = self.find_and_remove_call(uid).unwrap();

                    tracing::warn!(
                        "received stream packet for non-stream rpc: channel_id=0x{:02x}, service_id=0x{:08x}, method_id=0x{:08x}, call_id=0x{:02x}",
                        packet.channel_id, packet.service_id, packet.method_id, packet.call_id
                    );

                    call.complete_with_error(Status::InvalidArgument).await;
                    self.send_client_error(uid, Status::InvalidArgument).await?;
                }
            },
            None => {               // no pending call found, try to notify server
                tracing::debug!(
                    "received stream packet for non-pending rpc: service_id=0x{:08x}, method_id=0x{:08x}, call_id=0x{:02x}",
                    packet.service_id, packet.method_id, packet.call_id
                );

                self.send_client_error(uid, Status::FailedPrecondition).await?;
            },
        }

        Ok(())
    }

    async fn process_request(&mut self, request: CallRequest) -> Result<(), Error> {
        match request {
            CallRequest::New { ty, uid, payload, sender, tx } => {
                let call = Call { ty, uid, sender };

                let packet = RpcPacket {
                    r#type: PacketType::Request.into(),
                    channel_id: uid.channel,
                    service_id: uid.service,
                    method_id: uid.method,
                    payload,
                    status: Status::Ok as _,
                    call_id: uid.call,
                };

                let action = if tx { "starting" } else { "opening" };
                tracing::trace!(
                    "{} rpc: channel_id=0x{:02x}, service_id=0x{:08x}, method_id=0x{:08x}, call_id=0x{:02x}",
                    action, packet.channel_id, packet.service_id, packet.method_id, packet.call_id,
                );

                self.pending.push(call);
                if tx {
                    self.send(packet).await?;
                }

                Ok(())
            },
            CallRequest::Error { uid, code, tx } => {
                match self.find_and_remove_call(uid) {
                    Some(mut call) => {
                        tracing::trace!(
                            "cancelling active rpc with code: channel_id=0x{:02x}, service_id=0x{:08x}, method_id=0x{:08x}, call_id=0x{:02x}, code={}",
                            uid.channel, uid.service, uid.method, uid.call, code as u32,
                        );

                        call.complete_with_error(code).await;
                        if tx {
                            self.send_client_error(uid, code).await?;
                        }

                        Ok(())
                    },
                    None => {
                        tracing::trace!(
                            "received error request for non-pending rpc: channel_id=0x{:02x}, service_id=0x{:08x}, method_id=0x{:08x}, call_id=0x{:02x}, code={}",
                            uid.channel, uid.service, uid.method, uid.call, code as u32,
                        );
                        Ok(())
                    },
                }
            },
        }
    }

    fn find_and_remove_call(&mut self, uid: CallUid) -> Option<Call> {
        let index = self.pending.iter().position(|call| call.uid == uid);

        match index {
            Some(index) => Some(self.pending.remove(index)),
            None => None,
        }
    }

    fn find_call_mut(&mut self, uid: CallUid) -> Option<&mut Call> {
        self.pending.iter_mut().find(|call| call.uid == uid)
    }

    async fn send_client_error(&mut self, uid: CallUid, status: Status) -> Result<(), Error> {
        let status: u32 = status.into();

        tracing::trace!(
            "sending client error packet: status={}, channel_id=0x{:02x}, service_id=0x{:08x}, method_id=0x{:08x}, call_id=0x{:02x}",
            status, uid.channel, uid.service, uid.method, uid.call,
        );

        let error_packet = RpcPacket {
            r#type: PacketType::ClientError as _,
            channel_id: uid.channel,
            service_id: uid.service,
            method_id: uid.method,
            call_id: uid.call,
            payload: Vec::new(),
            status,
        };

        self.send(error_packet).await
    }

    async fn send(&mut self, packet: RpcPacket) -> Result<(), Error> {
        self.io_tx.send(packet).await?;
        Ok(())
    }
}


#[derive(Debug, Clone)]
pub struct ClientHandle {
    queue_tx: mpsc::UnboundedSender<CallRequest>,
}

impl ClientHandle {
    pub fn call_unary<M1, M2>(&mut self, request: Request<M1>) -> Result<UnaryResponse<M2>, Error>
    where
        M1: Message,
        M2: Message + Default,
    {
        let handle = self.call(RpcType::Unary, request)?;

        let response = UnaryResponse {
            maker: std::marker::PhantomData,
            handle,
        };

        Ok(response)
    }

    pub fn call_server_stream<M1, M2>(&mut self, request: Request<M1>) -> Result<StreamResponse<M2>, Error>
    where
        M1: Message,
        M2: Message + Default,
    {
        let handle = self.call(RpcType::ServerStream, request)?;

        let stream = StreamResponse {
            marker: std::marker::PhantomData,
            handle,
        };

        Ok(stream)
    }

    fn call<M>(&mut self, ty: RpcType, request: Request<M>) -> Result<CallHandle, Error>
    where
        M: Message,
    {
        let (sender, receiver) = mpsc::unbounded();

        let uid = CallUid {
            channel: request.channel_id,
            service: request.service_id,
            method: request.method_id,
            call: request.call_id,
        };

        let payload = request.message.encode_to_vec();
        let queue_tx = self.queue_tx.clone();

        let request = CallRequest::New { ty, uid, payload, sender, tx: true };
        let handle = CallHandle { uid, queue_tx, receiver, cancel_on_drop: true };

        self.queue_tx.unbounded_send(request)
            .map_err(|_| Error::aborted("the channel has been closed, no new calls are allowed"))?;

        Ok(handle)
    }

    pub fn open_unary<M>(&mut self, request: Request<()>) -> Result<UnaryResponse<M>, Error>
    where
        M: Message + Default,
    {
        let handle = self.open(RpcType::Unary, request)?;

        let response = UnaryResponse {
            maker: std::marker::PhantomData,
            handle,
        };

        Ok(response)
    }

    pub fn open_server_stream<M>(&mut self, request: Request<()>) -> Result<StreamResponse<M>, Error>
    where
        M: Message + Default,
    {
        let handle = self.open(RpcType::ServerStream, request)?;

        let stream = StreamResponse {
            marker: std::marker::PhantomData,
            handle,
        };

        Ok(stream)
    }

    fn open<M>(&mut self, ty: RpcType, request: Request<M>) -> Result<CallHandle, Error>
    where
        M: Message,
    {
        let (sender, receiver) = mpsc::unbounded();

        let uid = CallUid {
            channel: request.channel_id,
            service: request.service_id,
            method: request.method_id,
            call: request.call_id,
        };

        let payload = Vec::new();
        let queue_tx = self.queue_tx.clone();

        let request = CallRequest::New { ty, uid, payload, sender, tx: false };
        let handle = CallHandle { uid, queue_tx, receiver, cancel_on_drop: false };

        self.queue_tx.unbounded_send(request)
            .map_err(|_| Error::aborted("the channel has been closed, no new calls are allowed"))?;

        Ok(handle)
    }
}


#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct CallUid {
    channel: u32,
    service: u32,
    method: u32,
    call: u32,
}

impl CallUid {
    fn from_packet(packet: &RpcPacket) -> Self {
        Self {
            channel: packet.channel_id,
            service: packet.service_id,
            method: packet.method_id,
            call: packet.call_id
        }
    }
}


#[derive(Debug)]
enum CallRequest {
    New {
        ty: RpcType,
        uid: CallUid,
        payload: Vec<u8>,
        sender: mpsc::UnboundedSender<CallUpdate>,
        tx: bool,
    },
    Error {
        uid: CallUid,
        code: Status,
        tx: bool,
    },
}


#[derive(Debug)]
enum CallUpdate {
    Complete {
        data: Vec<u8>,
        status: Status,
    },
    StreamItem {
        data: Vec<u8>,
    },
    Error {
        status: Status,
    }
}


#[derive(Debug)]
struct Call {
    ty: RpcType,
    uid: CallUid,
    sender: mpsc::UnboundedSender<CallUpdate>,
}

impl Call {
    pub async fn complete(&mut self, payload: Vec<u8>, status: Status) {
        let update = CallUpdate::Complete { data: payload, status };
        self.push_update(update).await;
        self.sender.close_channel();
    }

    pub async fn complete_with_error(&mut self, status: Status) {
        let update = CallUpdate::Error { status };
        self.push_update(update).await;
        self.sender.close_channel();
    }

    pub async fn push_item(&mut self, payload: Vec<u8>) {
        let update = CallUpdate::StreamItem { data: payload };
        self.push_update(update).await;
    }

    async fn push_update(&mut self, update: CallUpdate) {
        if let Err(e) = self.sender.unbounded_send(update) {
            let update = e.into_inner();

            match update {
                CallUpdate::Complete { .. } => {
                    tracing::warn!(
                        "cannot send call update, caller is gone: channel_id=0x{:02x}, service_id=0x{:08x}, method_id=0x{:08x}, call_id=0x{:02x}, update=complete",
                        self.uid.channel, self.uid.service, self.uid.method, self.uid.call,
                    )
                },
                CallUpdate::StreamItem { .. } => {
                    tracing::warn!(
                        "cannot send call update, caller is gone: channel_id=0x{:02x}, service_id=0x{:08x}, method_id=0x{:08x}, call_id=0x{:02x}, update=stream",
                        self.uid.channel, self.uid.service, self.uid.method, self.uid.call,
                    )
                },
                CallUpdate::Error { status } => {
                    let code: u32 = status.into();

                    tracing::trace!(
                        "cannot send call update, caller is gone: channel_id=0x{:02x}, service_id=0x{:08x}, method_id=0x{:08x}, call_id=0x{:02x}, update=error, error={}",
                        self.uid.channel, self.uid.service, self.uid.method, self.uid.call, code,
                    )
                },
            }
        }
    }
}

impl Drop for Call {
    fn drop(&mut self) {
        // Notify caller that call has been aborted if the call has not been
        // completed yet. Ignore errors.
        if !self.sender.is_closed() {
            let update = CallUpdate::Error { status: Status::Aborted };
            let _ = self.sender.unbounded_send(update);
            self.sender.close_channel();
        }
    }
}


struct CallHandle {
    uid: CallUid,
    queue_tx: mpsc::UnboundedSender<CallRequest>,
    receiver: mpsc::UnboundedReceiver<CallUpdate>,
    cancel_on_drop: bool,
}

impl CallHandle {
    fn is_complete(&self) -> bool {
        self.queue_tx.is_closed()
    }

    fn error(&mut self, code: Status, tx: bool) -> bool {
        let request = CallRequest::Error { uid: self.uid, code, tx };
        let ok = self.queue_tx.unbounded_send(request).is_ok();

        // Sending an error will complete the RPC. Disconnect our queue end to
        // prevent more errors/cancel-requests to be sent.
        self.queue_tx.disconnect();

        ok
    }

    fn abandon(&mut self) -> bool {
        self.error(Status::Cancelled, false)
    }

    fn cancel_on_drop(&mut self, cancel: bool) {
        self.cancel_on_drop = cancel
    }

    fn cancel(&mut self) -> bool {
        self.error(Status::Cancelled, true)
    }

    async fn cancel_and_wait(&mut self) -> Result<(), Error> {
        if !self.cancel() {
            return Ok(())
        }

        loop {
            match self.receiver.next().await {
                Some(CallUpdate::StreamItem { .. }) => {
                    continue
                },
                Some(CallUpdate::Complete { .. }) => {
                    return Ok(())
                },
                Some(CallUpdate::Error { status: Status::Cancelled }) => {
                    return Ok(())
                },
                Some(CallUpdate::Error { status }) => {
                    return Err(Error::from(status))
                },
                None => {
                    return Ok(())
                },
            }
        }
    }
}

impl Drop for CallHandle {
    fn drop(&mut self) {
        if self.cancel_on_drop {
            self.cancel();
        } else {
            self.abandon();
        }
    }
}


pub struct Request<M> {
    pub channel_id: u32,
    pub service_id: u32,
    pub method_id: u32,
    pub call_id: u32,
    pub message: M,
}


pub struct UnaryResponse<M> {
    maker: std::marker::PhantomData<M>,
    handle: CallHandle,
}

impl<M> UnaryResponse<M>
where
    M: Message + Default,
{
    pub async fn result(&mut self) -> Result<M, Error> {
        let update = match self.handle.receiver.next().await {
            Some(update) => update,
            None => return Err(Error::resource_exhausted("cannot fetch result() multiple times")),
        };

        let data = match update {
            CallUpdate::Complete { data, status: Status::Ok } => data,
            CallUpdate::Complete { status, .. } => return Err(Error::from(status)),
            CallUpdate::Error { status } => return Err(Error::from(status)),
            CallUpdate::StreamItem { .. } => unreachable!("received stream update on unary rpc"),
        };

        self.handle.queue_tx.disconnect();

        let message = M::decode(&data[..])?;
        Ok(message)
    }

    pub fn abandon(&mut self) -> bool {
        self.handle.abandon()
    }

    pub fn cancel_on_drop(&mut self, cacnel: bool) {
        self.handle.cancel_on_drop(cacnel)
    }

    pub fn cancel(&mut self) -> bool {
        self.handle.cancel()
    }

    pub async fn cancel_and_wait(&mut self) -> Result<(), Error> {
        self.handle.cancel_and_wait().await
    }

    pub fn is_complete(&self) -> bool {
        self.handle.is_complete()
    }
}


pub struct StreamResponse<M> {
    marker: std::marker::PhantomData<M>,
    handle: CallHandle,
}

impl<M> StreamResponse<M>
where
    M: Message + Default,
{
    pub fn stream(&mut self) -> ServerStream<'_, M> {
        ServerStream {
            marker: std::marker::PhantomData,
            handle: &mut self.handle,
        }
    }

    pub fn abandon(&mut self) -> bool {
        self.handle.abandon()
    }

    pub fn cancel_on_drop(&mut self, cacnel: bool) {
        self.handle.cancel_on_drop(cacnel)
    }

    pub fn cancel(&mut self) -> bool {
        self.handle.cancel()
    }

    pub async fn cancel_and_wait(&mut self) -> Result<(), Error> {
        self.handle.cancel_and_wait().await
    }

    pub fn is_complete(&self) -> bool {
        self.handle.is_complete()
    }
}


pub struct ServerStream<'a, M> {
    marker: std::marker::PhantomData<&'a mut M>,
    handle: &'a mut CallHandle,
}

impl<'a, M> Stream for ServerStream<'a, M>
where
    M: Message + Default,
{
    type Item = Result<M, Error>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> Poll<Option<Self::Item>> {
        let update = match Pin::new(&mut self.handle.receiver).poll_next(cx) {
            Poll::Ready(Some(update)) => update,
            Poll::Ready(None) => return Poll::Ready(None),
            Poll::Pending => return Poll::Pending,
        };

        let data = match update {
            CallUpdate::StreamItem { data } => {
                data
            },
            CallUpdate::Complete { .. } => {
                // This indicates the end of the stream. The payload
                // should be empty.
                self.handle.receiver.close();
                self.handle.queue_tx.disconnect();
                return Poll::Ready(None);
            },
            CallUpdate::Error { status } => {
                self.handle.receiver.close();
                self.handle.queue_tx.disconnect();
                return Poll::Ready(Some(Err(Error::from(status))));
            },
        };

        let result = match M::decode(&data[..]) {
            Ok(message) => {
                Ok(message)
            },
            Err(e) => {
                self.handle.error(Status::InvalidArgument, true);
                Err(e.into())
            },
        };

        Poll::Ready(Some(result))
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.handle.receiver.size_hint()
    }
}

impl<'a, M> FusedStream for ServerStream<'a, M>
where
    M: Message + Default,
{
    fn is_terminated(&self) -> bool {
        self.handle.receiver.is_terminated()
    }
}


#[derive(Debug, Clone)]
pub struct UnaryRpc<M1, M2> {
    marker1: std::marker::PhantomData<*const M1>,
    marker2: std::marker::PhantomData<*const M2>,
    path: Path,
}

impl<M1, M2> UnaryRpc<M1, M2>
where
    M1: Message,
    M2: Message + Default,
{
    pub fn new(path: impl Into<Path>) -> Self {
        Self {
            marker1: std::marker::PhantomData,
            marker2: std::marker::PhantomData,
            path: path.into(),
        }
    }

    pub fn call(&self, handle: &mut ClientHandle, channel_id: u32, call_id: u32, message: M1)
        -> Result<UnaryResponse<M2>, Error>
    {
        let req = Request {
            channel_id,
            service_id: self.path.service().hash(),
            method_id: self.path.method().hash(),
            call_id,
            message,
        };

        handle.call_unary(req)
    }

    pub fn open(&self, handle: &mut ClientHandle, channel_id: u32, call_id: u32)
        -> Result<UnaryResponse<M2>, Error>
    {
        let req = Request {
            channel_id,
            service_id: self.path.service().hash(),
            method_id: self.path.method().hash(),
            call_id,
            message: (),
        };

        handle.open_unary(req)
    }
}


#[derive(Debug, Clone)]
pub struct ServerStreamRpc<M1, M2> {
    marker1: std::marker::PhantomData<*const M1>,
    marker2: std::marker::PhantomData<*const M2>,
    path: Path,
}

impl<M1, M2> ServerStreamRpc<M1, M2>
where
    M1: Message,
    M2: Message + Default,
{
    pub fn new(path: impl Into<Path>) -> Self {
        Self {
            marker1: std::marker::PhantomData,
            marker2: std::marker::PhantomData,
            path: path.into(),
        }
    }

    pub fn call(&self, handle: &mut ClientHandle, channel_id: u32, call_id: u32, message: M1)
        -> Result<StreamResponse<M2>, Error>
    {
        let req = Request {
            channel_id,
            service_id: self.path.service().hash(),
            method_id: self.path.method().hash(),
            call_id,
            message,
        };

        handle.call_server_stream(req)
    }

    pub fn open(&self, handle: &mut ClientHandle, channel_id: u32, call_id: u32)
        -> Result<StreamResponse<M2>, Error>
    {
        let req = Request {
            channel_id,
            service_id: self.path.service().hash(),
            method_id: self.path.method().hash(),
            call_id,
            message: (),
        };

        handle.open_server_stream(req)
    }
}
