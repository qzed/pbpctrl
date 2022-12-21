use std::pin::Pin;
use std::sync::Arc;
use std::task::Poll;

use futures::{Sink, SinkExt, Stream, StreamExt};
use futures::channel::mpsc;
use futures::lock::Mutex;
use futures::stream::{SplitSink, SplitStream, FusedStream};
use num_enum::FromPrimitive;

use crate::protocol::addr::Address;

use super::codec::Packet;
use super::types::{RpcStatus, RpcType, RpcPacket, PacketType};


pub struct Client<S> {
    receiver: SplitStream<S>,
    sender: Arc<Mutex<SplitSink<S, Packet>>>,
    state: Arc<Mutex<State>>,
}

impl<S, E> Client<S>
where
    S: Sink<Packet>,
    S: Stream<Item = Result<Packet, E>> + Unpin,
    S::Error: std::fmt::Debug,
    E: std::fmt::Debug,
{
    pub fn new(stream: S) -> Client<S> {
        let (sink, stream) = stream.split();

        let state = State {
            pending: Vec::new(),
        };

        Client {
            receiver: stream,
            sender: Arc::new(Mutex::new(sink)),
            state: Arc::new(Mutex::new(state)),
        }
    }

    pub fn handle(&self) -> ClientHandle<S> {
        ClientHandle {
            sender: self.sender.clone(),
            state: self.state.clone(),
        }
    }

    pub async fn run(&mut self) -> Result<(), E> {
        while let Some(packet) = self.receiver.next().await {
            self.process(packet?).await;
        }

        Ok(())
    }

    async fn process(&self, packet: Packet) {
        log::debug!(
            "received packet: type=0x{:02x}, channel_id=0x{:02x}, service_id=0x{:08x}, method_id=0x{:08x}, call_id=0x{:02x}",
            packet.rpc.r#type, packet.rpc.channel_id, packet.rpc.service_id, packet.rpc.method_id, packet.rpc.call_id
        );

        let ty = packet.rpc.r#type;
        let ty = PacketType::from_i32(ty);

        match ty {
            Some(PacketType::Response) => {
                self.complete(packet).await
            },
            Some(PacketType::ServerError) => {
                self.complete_with_error(packet).await
            },
            Some(PacketType::ServerStream) => {
                self.stream_push(packet).await
            },
            Some(_) => {
                log::error!(
                    "unsupported packet type: type=0x{:02x}, channel_id=0x{:02x}, service_id=0x{:08x}, method_id=0x{:08x}, call_id=0x{:02x}",
                    packet.rpc.r#type, packet.rpc.channel_id, packet.rpc.service_id, packet.rpc.method_id, packet.rpc.call_id
                );
            },
            None => {
                log::error!(
                    "unknown packet type: type=0x{:02x}, channel_id=0x{:02x}, service_id=0x{:08x}, method_id=0x{:08x}, call_id=0x{:02x}",
                    packet.rpc.r#type, packet.rpc.channel_id, packet.rpc.service_id, packet.rpc.method_id, packet.rpc.call_id
                );
            },
        }
    }

    async fn complete(&self, packet: Packet) {
        let call = {
            let mut state = self.state.lock().await;
            state.find_and_remove_call(&packet)
        };

        match call {
            Some(mut call) => {     // pending call found, complete rpc
                log::debug!(
                    "completing rpc: channel_id=0x{:02x}, service_id=0x{:08x}, method_id=0x{:08x}, call_id=0x{:02x}",
                    packet.rpc.channel_id, packet.rpc.service_id, packet.rpc.method_id, packet.rpc.call_id
                );

                let status = RpcStatus::from_primitive(packet.rpc.status);
                call.complete(packet.rpc.payload, status).await;
            },
            None => {               // no pending call found, silently drop packet
                log::warn!(
                    "received response for non-pending rpc: channel_id=0x{:02x}, service_id=0x{:08x}, method_id=0x{:08x}, call_id=0x{:02x}",
                    packet.rpc.channel_id, packet.rpc.service_id, packet.rpc.method_id, packet.rpc.call_id
                );
            },
        }
    }

    async fn complete_with_error(&self, packet: Packet) {
        let call = {
            let mut state = self.state.lock().await;
            state.find_and_remove_call(&packet)
        };

        match call {
            Some(mut call) => {     // pending call found, complete rpc with error
                log::debug!(
                    "completing rpc with error: channel_id=0x{:02x}, service_id=0x{:08x}, method_id=0x{:08x}, call_id=0x{:02x}, status={}",
                    packet.rpc.channel_id, packet.rpc.service_id, packet.rpc.method_id, packet.rpc.call_id, packet.rpc.status
                );

                let status = RpcStatus::from_primitive(packet.rpc.status);
                call.complete_with_error(status).await;
            },
            None => {               // no pending call found, silently drop packet
                log::warn!(
                    "received error for non-pending rpc: channel_id=0x{:02x}, service_id=0x{:08x}, method_id=0x{:08x}, call_id=0x{:02x}, status={}",
                    packet.rpc.channel_id, packet.rpc.service_id, packet.rpc.method_id, packet.rpc.call_id, packet.rpc.status
                );
            },
        }
    }

    async fn stream_push(&self, packet: Packet) {
        let mut state = self.state.lock().await;
        let call = state.find_call_mut(&packet);

        match call {
            Some(call) => {         // pending call found, forward packet to caller
                log::debug!(
                    "pushing server stream packet to caller: channel_id=0x{:02x}, service_id=0x{:08x}, method_id=0x{:08x}, call_id=0x{:02x}",
                    packet.rpc.channel_id, packet.rpc.service_id, packet.rpc.method_id, packet.rpc.call_id
                );

                if call.ty.has_server_stream() {    // packet was expected, forward it
                    call.push_item(packet.rpc.payload).await;
                } else {            // this type of rpc doesn't expect streaming packets from the server
                    // SAFETY: We are the only ones that can add, remove, or
                    //         otherwise modify items in-between the above find
                    //         operation and this one as we have the lock.
                    let mut call = state.find_and_remove_call(&packet).unwrap();
                    drop(state);

                    log::warn!(
                        "received stream packet for non-stream rpc: channel_id=0x{:02x}, service_id=0x{:08x}, method_id=0x{:08x}, call_id=0x{:02x}",
                        packet.rpc.channel_id, packet.rpc.service_id, packet.rpc.method_id, packet.rpc.call_id
                    );

                    self.try_send_client_error(&packet, RpcStatus::InvalidArgument).await;
                    call.complete_with_error(RpcStatus::InvalidArgument).await;
                }
            },
            None => {               // no pending call found, try to notify server
                drop(state);

                log::warn!(
                    "received stream packet for non-pending rpc: service_id=0x{:08x}, method_id=0x{:08x}, call_id=0x{:02x}",
                    packet.rpc.service_id, packet.rpc.method_id, packet.rpc.call_id
                );

                self.try_send_client_error(&packet, RpcStatus::FailedPrecondition).await;
            },
        }
    }

    async fn send(&self, packet: Packet) -> Result<(), S::Error> {
        let mut sink = self.sender.lock().await;
        sink.send(packet).await
    }

    async fn try_send_client_error(&self, packet: &Packet, status: RpcStatus) {
        let status: u32 = status.into();

        log::debug!(
            "sending client error packet: status={}, channel_id=0x{:02x}, service_id=0x{:08x}, method_id=0x{:08x}, call_id=0x{:02x}",
            status, packet.rpc.channel_id, packet.rpc.service_id, packet.rpc.method_id, packet.rpc.call_id
        );

        let addr = Address::from_value(packet.address).swap();

        let error_packet = Packet {
            address: addr.value(),
            rpc: RpcPacket {
                r#type: PacketType::ClientError as _,
                channel_id: packet.rpc.channel_id,
                service_id: packet.rpc.service_id,
                method_id: packet.rpc.method_id,
                call_id: packet.rpc.call_id,
                payload: Vec::new(),
                status,
            },
        };

        if let Err(e) = self.send(error_packet).await {
            log::error!(
                "error client error packet: status=0x{:02x}, channel_id=0x{:02x}, service_id=0x{:08x}, method_id=0x{:08x}, call_id=0x{:02x}: {:?}",
                status, packet.rpc.channel_id, packet.rpc.service_id, packet.rpc.method_id, packet.rpc.call_id, e
            );
        }
    }
}

impl<S> Drop for Client<S> {
    fn drop(&mut self) {
        // TODO: cancel all pending calls
    }
}


pub struct ClientHandle<S> {
    sender: Arc<Mutex<SplitSink<S, Packet>>>,
    state: Arc<Mutex<State>>,
}

impl<S, E> ClientHandle<S>
where
    S: Sink<Packet>,
    S: Stream<Item = Result<Packet, E>> + Unpin,
{
    pub async fn call(&self, ty: RpcType, request: Packet) -> Result<CallHandle, S::Error> {
        let (sender, receiver) = mpsc::unbounded();

        let handle = CallHandle {
            ty,
            receiver,
        };

        let call = Call {
            ty,
            channel_id: request.rpc.channel_id,
            service_id: request.rpc.service_id,
            method_id: request.rpc.method_id,
            call_id: request.rpc.call_id,
            sender,
        };

        {
            let mut state = self.state.lock().await;
            state.pending.push(call);
        }

        self.send(request).await?;
        Ok(handle)
    }

    async fn send(&self, packet: Packet) -> Result<(), S::Error> {
        let mut sink = self.sender.lock().await;
        sink.send(packet).await
    }
}


struct State {
    pending: Vec<Call>,
}

impl State {
    fn find_and_remove_call(&mut self, packet: &Packet) -> Option<Call> {
        let index = self.pending.iter().position(|call| {
            call.channel_id == packet.rpc.channel_id
                && call.service_id == packet.rpc.service_id
                && call.method_id == packet.rpc.method_id
                && call.call_id == packet.rpc.call_id
        });

        match index {
            Some(index) => Some(self.pending.remove(index)),
            None => None,
        }
    }

    fn find_call_mut(&mut self, packet: &Packet) -> Option<&mut Call> {
        self.pending.iter_mut().find(|call| {
            call.channel_id == packet.rpc.channel_id
                && call.service_id == packet.rpc.service_id
                && call.method_id == packet.rpc.method_id
                && call.call_id == packet.rpc.call_id
        })
    }
}


enum CallUpdate {
    Complete {
        data: Vec<u8>,
        status: RpcStatus,
    },
    StreamItem {
        data: Vec<u8>,
    },
    Error {
        status: RpcStatus,
    }
}


struct Call {
    ty: RpcType,

    channel_id: u32,
    service_id: u32,
    method_id: u32,
    call_id: u32,

    sender: mpsc::UnboundedSender<CallUpdate>,
}

impl Call {
    pub async fn complete(&mut self, payload: Vec<u8>, status: RpcStatus) {
        let update = CallUpdate::Complete { data: payload, status };
        self.push_update(update).await;
        self.sender.close_channel();
    }

    pub async fn complete_with_error(&mut self, status: RpcStatus) {
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
                    log::warn!(
                        "cannot send call update, caller is gone: channel_id=0x{:02x}, service_id=0x{:08x}, method_id=0x{:08x}, call_id=0x{:02x}, update=complete",
                        self.channel_id, self.service_id, self.method_id, self.call_id,
                    )
                },
                CallUpdate::StreamItem { .. } => {
                    log::warn!(
                        "cannot send call update, caller is gone: channel_id=0x{:02x}, service_id=0x{:08x}, method_id=0x{:08x}, call_id=0x{:02x}, update=stream",
                        self.channel_id, self.service_id, self.method_id, self.call_id,
                    )
                },
                CallUpdate::Error { status } => {
                    let code: u32 = status.into();

                    log::warn!(
                        "cannot send call update, caller is gone: channel_id=0x{:02x}, service_id=0x{:08x}, method_id=0x{:08x}, call_id=0x{:02x}, update=error, error={}",
                        self.channel_id, self.service_id, self.method_id, self.call_id, code,
                    )
                },
            }
        }
    }
}


pub struct CallHandle {
    ty: RpcType,
    receiver: mpsc::UnboundedReceiver<CallUpdate>,
}

impl CallHandle {
    pub async fn result(&mut self) -> Result<Vec<u8>, Error> {
        if self.ty != RpcType::Unary {
            panic!("called result() on stream");
        }

        let update = match self.receiver.next().await {
            Some(update) => update,
            None => return Err(Error::ResourceExhausted),
        };

        match update {
            CallUpdate::Complete { data, status: RpcStatus::Ok } => Ok(data),
            CallUpdate::Complete { status, .. } => Err(status),
            CallUpdate::Error { status } => Err(status),
            CallUpdate::StreamItem { .. } => unreachable!(),
        }
    }

    pub fn stream(&mut self) -> ServerStream<'_> {
        if !self.ty.has_server_stream() {
            panic!("called stream() on non-stream rpc");
        }

        ServerStream { call: self }
    }
}

impl Drop for CallHandle {
    fn drop(&mut self) {
        // TODO: cancel/abort this call?
    }
}


pub struct ServerStream<'a> {
    call: &'a mut CallHandle,
}

impl<'a> Stream for ServerStream<'a> {
    type Item = Result<Vec<u8>, Error>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> Poll<Option<Self::Item>> {
        let update = match Pin::new(&mut self.call.receiver).poll_next(cx) {
            Poll::Ready(Some(update)) => update,
            Poll::Ready(None) => return Poll::Ready(None),
            Poll::Pending => return Poll::Pending,
        };

        match update {
            CallUpdate::StreamItem { data } => {
                Poll::Ready(Some(Ok(data)))
            },
            CallUpdate::Complete { .. } => {
                // This indicates the end of the stream. The payload
                // should be empty.
                self.call.receiver.close();
                Poll::Ready(None)
            },
            CallUpdate::Error { status } => {
                self.call.receiver.close();
                Poll::Ready(Some(Err(status)))
            },
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.call.receiver.size_hint()
    }
}

impl<'a> FusedStream for ServerStream<'a> {
    fn is_terminated(&self) -> bool {
        self.call.receiver.is_terminated()
    }
}


pub type Error = RpcStatus;
