use std::sync::OnceLock;

use super::Message;
use futures::StreamExt;
use futures::channel::mpsc::{Receiver, Sender};
use std::sync::RwLock;

use std::sync::Arc;
use std::sync::LazyLock;
use std::sync::atomic::AtomicBool;
pub static INIT_STATUS: LazyLock<Arc<AtomicBool>> =
    LazyLock::new(|| Arc::new(AtomicBool::new(false)));

#[derive(Debug, Clone)]
pub enum PasswordMessage {
    Pw(String),
    Canceled,
}

static POLKIT_SENDER: OnceLock<Sender<Message>> = OnceLock::new();

static POLKIT_PASSWORD_RECEIVER: OnceLock<RwLock<Receiver<PasswordMessage>>> = OnceLock::new();

pub fn init_sender(sender: Sender<Message>) {
    POLKIT_SENDER.set(sender).expect("Should be set only once")
}

pub fn get_sender() -> Sender<Message> {
    POLKIT_SENDER.get().expect("Should be inited").clone()
}

pub fn init_pw_receiver(receiver: Receiver<PasswordMessage>) {
    POLKIT_PASSWORD_RECEIVER
        .set(RwLock::new(receiver))
        .expect("Should be set only once")
}

pub async fn receive_pw_next() -> Option<PasswordMessage> {
    let receiver = POLKIT_PASSWORD_RECEIVER.get().expect("Should be inited");
    let mut receiver = receiver.write().unwrap();
    receiver.next().await
}
