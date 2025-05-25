use glib::error::ErrorDomain;
use glib::object::Cast;
use glib::subclass::prelude::*;
use polkit_agent_rs::Session as AgentSession;
use polkit_agent_rs::gio;
use polkit_agent_rs::polkit;
use polkit_agent_rs::polkit::UnixUser;
use polkit_agent_rs::subclass::ListenerImpl;

use crate::Message;
use crate::global;
use global::PasswordMessage;
#[derive(Default)]
pub struct MyPolkit {}
use std::sync::Arc;
use std::sync::atomic::AtomicU8;

#[derive(Debug, Clone, Copy)]
struct SessionError;

impl ErrorDomain for SessionError {
    fn domain() -> glib::Quark {
        glib::Quark::from_str("session_error")
    }
    fn code(self) -> i32 {
        -1
    }
    fn from(code: i32) -> Option<Self>
    where
        Self: Sized,
    {
        if code == -1 {
            return Some(Self);
        }
        None
    }
}

fn start_session(
    session: &AgentSession,
    name: String,
    cancellable: gio::Cancellable,
    task: gio::Task<String>,
    cookie: String,
    count: Arc<AtomicU8>,
) {
    let sub_loop = glib::MainLoop::new(None, true);
    let name2 = name.clone();
    let cancellable2 = cancellable.clone();
    let task2 = task.clone();

    let sub_loop_2 = sub_loop.clone();
    let sub_loop_3 = sub_loop.clone();
    session.connect_completed(move |_session, success| {
        let name2 = name2.clone();
        let cancellable2 = cancellable2.clone();
        let task = task.clone();
        let cookie = cookie.clone();
        let count = count.clone();
        let mut sender = global::get_sender();
        if !success {
            count.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            if count.load(std::sync::atomic::Ordering::Relaxed) >= 3 {
                unsafe {
                    task.return_result(Err(glib::Error::new(
                        SessionError,
                        "You have used all attempts",
                    )));
                }
                sub_loop_2.quit();
                let _ = sender.try_send(Message::PolkitComplete);
                return;
            }
            let user: UnixUser = UnixUser::new_for_name(&name2).unwrap();
            let session = AgentSession::new(&user, &cookie);
            start_session(&session, name2, cancellable2, task, cookie, count);
            sub_loop_2.quit();
            return;
        } else {
            unsafe {
                task.return_result(Ok("success".to_string()));
            }
        }
        let _ = sender.try_send(Message::PolkitComplete);

        sub_loop_2.quit();
    });
    session.connect_show_info(|_session, info| {
        let mut sender = global::get_sender().clone();
        let _ = sender.try_send(Message::PolkitInfo(info.to_owned()));
    });
    session.connect_show_error(|_session, error| {
        let mut sender = global::get_sender().clone();
        let _ = sender.try_send(Message::PolkitError(error.to_owned()));
    });
    session.connect_request(move |session, request, _echo_on| {
        if !request.starts_with("Password:") {
            return;
        }
        let mut sender = global::get_sender();
        let _ = sender.try_send(Message::PolkitCome);
        let task = task2.clone();
        let context = sub_loop_3.context();
        let rt = context.block_on(async { global::receive_pw_next().await });

        if let Some(PasswordMessage::Pw(pw)) = rt {
            session.response(&pw);
        } else {
            unsafe {
                task.return_result(Err(glib::Error::new(
                    SessionError,
                    "Do not accept the password",
                )));
            }
            let _ = sender.try_send(Message::PolkitComplete);
            sub_loop_3.quit();
        }
    });
    session.initiate();
    sub_loop.run();
}

impl ListenerImpl for MyPolkit {
    type Message = String;
    fn initiate_authentication(
        &self,
        _action_id: &str,
        _message: &str,
        _icon_name: &str,
        _details: &polkit::Details,
        cookie: &str,
        identities: Vec<polkit::Identity>,
        cancellable: gio::Cancellable,
        task: gio::Task<Self::Message>,
    ) {
        let users: Vec<UnixUser> = identities
            .into_iter()
            .flat_map(|idenifier| idenifier.dynamic_cast())
            .collect();

        let user0 = &users[0];
        let name = user0.name().unwrap();
        let session = AgentSession::new(user0, cookie);

        let count = Arc::new(AtomicU8::new(0));
        start_session(
            &session,
            name.to_string(),
            cancellable,
            task,
            cookie.to_string(),
            count,
        );
    }
    fn initiate_authentication_finish(
        &self,
        gio_result: Result<gio::Task<Self::Message>, glib::Error>,
    ) -> bool {
        match gio_result {
            Ok(_) => true,
            Err(err) => {
                println!("err: {err:?}");
                false
            }
        }
    }
}

#[glib::object_subclass]
impl ObjectSubclass for MyPolkit {
    const NAME: &'static str = "MyPolkit";
    type Type = super::MyPolkit;
    type ParentType = super::Listener;
}

impl ObjectImpl for MyPolkit {}
