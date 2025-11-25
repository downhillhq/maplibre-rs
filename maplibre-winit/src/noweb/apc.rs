use std::{
    cell::RefCell,
    marker::PhantomData,
    sync::mpsc::{self, Receiver, Sender},
    vec::IntoIter,
};

use maplibre::{
    environment::{OffscreenKernel, OffscreenKernelConfig},
    io::{
        apc::{AsyncProcedure, AsyncProcedureCall, CallError, Context, Input, IntoMessage, Message, SchedulerContext, SendError},
        scheduler::Scheduler,
    },
};

use crate::WinitEventLoopProxy;

#[derive(Clone)]
pub struct WinitContext {
    sender: Sender<Message>,
    proxy: WinitEventLoopProxy<()>,
}

impl Context for WinitContext {
    fn send_back<T: IntoMessage>(&self, message: T) -> Result<(), SendError> {
        self.sender
            .send(message.into())
            .map_err(|_e| SendError::Transmission)?;
        let _ = self.proxy.send_event(()); // Ignore error if loop is closed
        Ok(())
    }
}

pub struct WinitAsyncProcedureCall<K: OffscreenKernel, S: Scheduler> {
    channel: (Sender<Message>, Receiver<Message>),
    buffer: RefCell<Vec<Message>>,
    scheduler: S,
    phantom_k: PhantomData<K>,
    offscreen_kernel_config: OffscreenKernelConfig,
    proxy: WinitEventLoopProxy<()>,
}

impl<K: OffscreenKernel, S: Scheduler> WinitAsyncProcedureCall<K, S> {
    pub fn new(
        scheduler: S,
        offscreen_kernel_config: OffscreenKernelConfig,
        proxy: WinitEventLoopProxy<()>,
    ) -> Self {
        Self {
            channel: mpsc::channel(),
            buffer: RefCell::new(Vec::new()),
            phantom_k: PhantomData::default(),
            scheduler,
            offscreen_kernel_config,
            proxy,
        }
    }
}

impl<K: OffscreenKernel, S: Scheduler> AsyncProcedureCall<K> for WinitAsyncProcedureCall<K, S> {
    type Context = WinitContext;
    type ReceiveIterator<F: FnMut(&Message) -> bool> = IntoIter<Message>;

    fn receive<F: FnMut(&Message) -> bool>(&self, mut filter: F) -> Self::ReceiveIterator<F> {
        let mut buffer = self.buffer.borrow_mut();
        let mut ret = Vec::new();

        let mut index = 0usize;
        let mut max_len = buffer.len();
        while index < max_len {
            if filter(&buffer[index]) {
                ret.push(buffer.swap_remove(index));
                max_len -= 1;
            } else {
                index += 1;
            }
        }

        while let Ok(message) = self.channel.1.try_recv() {
            log::info!("Data reached main thread with tag: {:?}", message.tag());

            if filter(&message) {
                ret.push(message);
            } else {
                log::warn!("Message filtered out: {:?}", message.tag());
                buffer.push(message)
            }
        }

        ret.into_iter()
    }

    fn call(
        &self,
        input: Input,
        procedure: AsyncProcedure<K, Self::Context>,
    ) -> Result<(), CallError> {
        let sender = self.channel.0.clone();
        let proxy = self.proxy.clone();
        let offscreen_kernel_config = self.offscreen_kernel_config.clone();

        self.scheduler
            .schedule(move || async move {
                log::info!("Processing on thread: {:?}", std::thread::current().name());

                let kernel = K::create(offscreen_kernel_config);
                procedure(input, WinitContext { sender, proxy }, kernel)
                    .await
                    .unwrap();
            })
            .map_err(|_e| CallError::Schedule)
    }
}

