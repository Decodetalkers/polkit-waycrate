use futures::SinkExt;
use global::PasswordMessage;
use iced::Element;
use iced::Task;
use iced::widget::button;
use iced::window::Id;
use iced::{
    Alignment, Length,
    widget::{Space, column, container, row, text, text_input},
};
use iced_layershell::settings::LayerShellSettings;
use iced_layershell::settings::StartMode;
use iced_layershell::to_layer_message;
use iced_layershell::{
    daemon,
    reexport::NewLayerShellSettings,
    reexport::{Anchor, Layer},
};
use mypolkit::MyPolkit;

mod global;
mod mypolkit;

use futures::channel::mpsc::Sender;

#[to_layer_message(multi)]
#[derive(Debug, Clone)]
enum Message {
    PolkitCome,
    PolkitComplete,
    PolkitError(String),
    PolkitInfo(String),
    Cancel,
    Exit,
    Confirm,
    Opened(Id),
    PolkitPwSender(Sender<PasswordMessage>),
    PasswordChange(String),
}

#[derive(Debug)]
struct PolkitDialog {
    error_message: String,
    info_message: String,
    password: String,
    current_id: Option<Id>,
    #[allow(unused)]
    controller: glib::MainLoop,
    pw_sender: Option<Sender<PasswordMessage>>,
}

const DIALOG_NAMESPACE: &str = "polkit";

impl PolkitDialog {
    fn new(controller: glib::MainLoop) -> Self {
        Self {
            error_message: "".to_owned(),
            info_message: "".to_owned(),
            password: "".to_owned(),
            current_id: None,
            controller,
            pw_sender: None,
        }
    }
}

fn update(dialog: &mut PolkitDialog, message: Message) -> Task<Message> {
    use iced::window::Action as WindowAction;
    use iced_runtime::Action;
    match message {
        Message::PolkitCome => {
            if dialog.current_id.is_some() {
                return Task::none();
            }
            Task::done(Message::NewLayerShell {
                settings: NewLayerShellSettings {
                    size: None,
                    exclusive_zone: Some(-1),
                    anchor: Anchor::all(),
                    layer: Layer::Top,
                    use_last_output: true,
                    ..Default::default()
                },
                id: iced::window::Id::unique(),
            })
        }
        Message::Opened(id) => {
            dialog.current_id = Some(id);
            Task::none()
        }
        Message::Exit => {
            dialog.controller.quit();
            iced_runtime::task::effect(Action::Exit)
        }
        Message::Confirm => {
            if let Some(pw_sender) = &mut dialog.pw_sender {
                let _ = pw_sender.try_send(PasswordMessage::Pw(dialog.password.clone()));
            }
            Task::none()
        }
        Message::PolkitComplete => {
            let id = dialog.current_id.take().unwrap();
            iced_runtime::task::effect(Action::Window(WindowAction::Close(id)))
        }
        Message::Cancel => {
            if let Some(pw_sender) = &mut dialog.pw_sender {
                let _ = pw_sender.try_send(PasswordMessage::Canceled);
            }
            Task::none()
        }
        Message::PolkitError(err) => {
            dialog.error_message = err;
            Task::none()
        }
        Message::PolkitInfo(info) => {
            dialog.info_message = info;
            Task::none()
        }

        Message::PasswordChange(password) => {
            dialog.password = password;
            Task::none()
        }
        Message::PolkitPwSender(sender) => {
            dialog.pw_sender = Some(sender);
            Task::none()
        }
        _ => Task::none(),
    }
}

fn view(dialog: &PolkitDialog, _id: iced::window::Id) -> Element<Message> {
    container(
        column![
            text_input("Your password", &dialog.password)
                .padding(10)
                .secure(true)
                .on_input(Message::PasswordChange),
            text(&dialog.info_message),
            text(&dialog.error_message),
            row![
                button("Confirm").on_press(Message::Confirm),
                Space::with_width(30.),
                button("Cancel").on_press(Message::Cancel),
                Space::with_width(30.),
                button("Debug").on_press(Message::Exit)
            ]
        ]
        .align_x(Alignment::Center)
        .width(Length::Fixed(700.)),
    )
    .center_x(Length::Fill)
    .center_y(Length::Fill)
    .height(Length::Fill)
    .into()
}

fn style(_dialog: &PolkitDialog, theme: &iced::Theme) -> iced::theme::Style {
    use iced::theme::Style;
    Style {
        background_color: iced::Color::from_rgba(0.2, 0.2, 0.2, 0.8),
        text_color: theme.palette().text,
    }
}

fn subscription(_dialog: &PolkitDialog) -> iced::Subscription<Message> {
    iced::Subscription::batch(vec![
        iced::window::open_events().map(Message::Opened),
        polkit_subscription(),
    ])
}

fn polkit_subscription() -> iced::Subscription<Message> {
    iced::Subscription::run(|| {
        iced::stream::channel(
            100,
            |sender: futures::channel::mpsc::Sender<Message>| async move {
                let (sender_pw, receiver_pw) = futures::channel::mpsc::channel(1000);
                global::init_pw_receiver(receiver_pw);
                let mut sender2 = sender.clone();
                global::init_sender(sender);
                sender2
                    .send(Message::PolkitPwSender(sender_pw))
                    .await
                    .unwrap();
                global::INIT_STATUS.store(true, std::sync::atomic::Ordering::Relaxed);
            },
        )
    })
}

const OBJECT_PATH: &str = "/org/waycrate/PolicyKit1/AuthenticationAgent";

fn run_polkit_thread() -> (glib::MainLoop, std::thread::JoinHandle<()>) {
    use polkit_agent_rs::RegisterFlags;
    use polkit_agent_rs::gio;
    use polkit_agent_rs::polkit::UnixSession;
    use polkit_agent_rs::traits::ListenerExt;
    let main_loop = glib::MainLoop::new(None, true);
    let main_loop_control = main_loop.clone();
    let thread = std::thread::spawn(move || {
        while !global::INIT_STATUS.load(std::sync::atomic::Ordering::Relaxed) {
            std::thread::sleep(std::time::Duration::from_secs(1));
        }
        let my_polkit = MyPolkit::default();

        let Ok(subject) = UnixSession::new_for_process_sync(
            nix::unistd::getpid().as_raw(),
            gio::Cancellable::NONE,
        ) else {
            return;
        };
        let _register = my_polkit
            .register(
                RegisterFlags::NONE,
                &subject,
                OBJECT_PATH,
                gio::Cancellable::NONE,
            )
            .unwrap();
        main_loop.run();
    });
    (main_loop_control, thread)
}

fn main() -> iced_layershell::Result {
    let (controller, handle) = run_polkit_thread();
    daemon(
        move || PolkitDialog::new(controller.clone()),
        DIALOG_NAMESPACE,
        update,
        view,
    )
    .layer_settings(LayerShellSettings {
        start_mode: StartMode::Background,
        exclusive_zone: -1,
        anchor: Anchor::all(),
        ..Default::default()
    })
    .subscription(subscription)
    .style(style)
    .run()?;
    let _ = handle.join();
    Ok(())
}
