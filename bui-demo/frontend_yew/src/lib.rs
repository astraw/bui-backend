use wasm_bindgen::prelude::*;

use yew::events::KeyboardEvent;
use yew::format::Json;
use yew::prelude::*;

use yew::services::fetch::{Credentials, FetchOptions, FetchService, FetchTask, Request, Response};

use yew::services::websocket::{WebSocketService, WebSocketStatus, WebSocketTask};

use bui_demo_data::{Callback, Shared};

pub struct App {
    link: ComponentLink<Self>,
    shared: Option<Shared>,
    es: WebSocketTask,
    ft: Option<FetchTask>,
    local_name: String,
}

pub enum Msg {
    /// We got new data from the backend.
    EsReady(Result<Shared, anyhow::Error>),
    /// Trigger a check of the web socket state.
    EsCheckState,
    /// Update our local copy of our name. (E.g. the user typed a key.)
    UpdateName(String),
    /// We want to update name on the server. (E.g. the user pressed Enter.)
    SendName,
    /// We want to update the recording status on the server. (E.g. the user clicked the button.)
    ToggleRecording,
    Ignore,
}

impl Component for App {
    type Message = Msg;
    type Properties = ();

    fn create(_: Self::Properties, link: ComponentLink<Self>) -> Self {
        let task = {
            let callback = link.callback(|Json(data)| Msg::EsReady(data));
            let notification = link.callback(|status| {
                if status == WebSocketStatus::Error {
                    log::error!("web socket error");
                }
                Msg::EsCheckState
            });
            let mut task =
                WebSocketService::connect_text("ws://localhost:3410/ws", callback, notification)
                    .unwrap();
            task
        };

        Self {
            link,
            shared: None,
            es: task,
            ft: None,
            local_name: "".to_string(),
        }
    }

    fn update(&mut self, msg: Self::Message) -> ShouldRender {
        match msg {
            Msg::EsReady(response) => {
                match response {
                    Ok(data_result) => {
                        self.shared = Some(data_result);
                    }
                    Err(e) => {
                        log::error!("{}", e);
                    }
                };
            }
            Msg::EsCheckState => {
                return true;
            }
            Msg::UpdateName(name) => {
                self.local_name = name;
            }
            Msg::SendName => {
                let name = self.local_name.clone();
                self.ft = self.send_message(&Callback::SetName(name));
                return false; // Don't update DOM, do that when backend notifies us of new state.
            }
            Msg::ToggleRecording => {
                let new_value = if let Some(ref shared) = self.shared {
                    !shared.is_recording
                } else {
                    false
                };
                self.ft = self.send_message(&Callback::SetIsRecording(new_value));
                return false; // Don't update DOM, do that when backend notifies us of new state.
            }
            Msg::Ignore => {
                return false;
            }
        }
        true
    }

    fn change(&mut self, _: Self::Properties) -> ShouldRender {
        false
    }

    fn view(&self) -> Html {
        html! {
            <div>
                { self.view_ready_state() }
                { self.view_shared() }
                { self.view_input() }
                <button onclick=self.link.callback(|_| Msg::ToggleRecording),>{ "Toggle recording" }</button>
            </div>
        }
    }
}

impl App {
    fn view_ready_state(&self) -> Html {
        // <p>{ format!("Connection State: {:?}", self.es.ready_state()) }</p>
        html! {
            <div></div>
        }
    }

    fn view_shared(&self) -> Html {
        if let Some(ref value) = self.shared {
            html! {
                <p>{ format!("{:?}", value) }</p>
            }
        } else {
            html! {
                <p>{ "Data hasn't fetched yet." }</p>
            }
        }
    }

    fn view_input(&self) -> Html {
        html! {
            <input placeholder="name",
                   value=&self.local_name,
                   oninput=self.link.callback(|e: InputData| Msg::UpdateName(e.value)),
                   onblur=self.link.callback(move|_| Msg::SendName),
                   onkeypress=self.link.callback(|e: KeyboardEvent| {
                       if e.key() == "Enter" { Msg::SendName } else { Msg::Ignore }
                   }), />
        }
    }

    fn send_message(&mut self, msg: &Callback) -> Option<yew::services::fetch::FetchTask> {
        let post_request = Request::post("callback")
            .header("Content-Type", "application/json;charset=UTF-8")
            .body(Json(msg))
            .expect("Failed to build request.");
        let callback = self
            .link
            .callback(move |resp: Response<Result<String, _>>| {
                match resp.body() {
                    &Ok(ref _s) => {}
                    &Err(ref e) => {
                        log::error!("Error when sending message: {:?}", e);
                    }
                }
                Msg::Ignore
            });
        let mut options = FetchOptions::default();
        options.credentials = Some(Credentials::SameOrigin);
        match FetchService::fetch_with_options(post_request, options, callback) {
            Ok(task) => Some(task),
            Err(err) => {
                log::error!("sending message failed with error: {}", err);
                None
            }
        }
    }
}

#[wasm_bindgen(start)]
pub fn run_app() {
    wasm_logger::init(wasm_logger::Config::default());
    yew::start_app::<App>();
}
