#![recursion_limit="256"]

#[macro_use]
extern crate yew;
#[macro_use]
extern crate stdweb;
extern crate serde;
#[macro_use]
extern crate serde_json;
extern crate bui_demo_data;
#[macro_use]
extern crate failure;
extern crate http;

use bui_demo_data::Shared;
use yew::prelude::*;
use yew::format::Json;
use yew::services::Task;

// Services we have defined as modules.
mod fetch;
use fetch::{FetchService, FetchTask, Request, Response, Credentials};

mod eventsource;
use eventsource::{ReadyState, EventSourceService, EventSourceTask};

struct Context {
    es: EventSourceService,
    web: FetchService,
}

struct Model {
    shared: Option<Shared>,
    es: Option<EventSourceTask>,
    ft: Option<FetchTask>,
    local_name: String,
    connection_state: ReadyState,
}

pub enum EventSourceAction {
    Connect,
    Disconnect,
    Lost(ReadyState),
}

enum Msg {
    EventSourceAction(EventSourceAction),
    EsReady(Result<Shared, failure::Error>),
    UpdateName(String),
    SendName,
    ToggleRecording,
    UpdateConnectionState(ReadyState),
    Ignore,
}

impl Component<Context> for Model {
    type Msg = Msg;
    type Properties = ();

    fn create(_: Self::Properties, context: &mut Env<Context, Self>) -> Self {
        let mut result = Self {
            shared: None,
            es: None,
            ft: None,
            local_name: "".to_string(),
            connection_state: ReadyState::Connecting,
        };
        // trigger connection on creation
        let msg = Msg::EventSourceAction(EventSourceAction::Connect);
        result.update(msg,context);
        result
    }

    fn update(&mut self, msg: Self::Msg, context: &mut Env<Context, Self>) -> ShouldRender {
        match msg {
            Msg::EventSourceAction(action) => {
                match action {
                    EventSourceAction::Connect => {
                        let callback = context.send_back(|Json(data): Json<Result<Shared, failure::Error>>| {
                            Msg::EsReady(data)
                        });
                        let notification = context.send_back(|status: ReadyState| {
                            match status {
                                ReadyState::Connecting => Msg::UpdateConnectionState(status),
                                ReadyState::Open => Msg::UpdateConnectionState(status),
                                ReadyState::Closed => Msg::EventSourceAction(EventSourceAction::Lost(status)),
                            }
                        });
                        let task = context.es.connect("events", "bui_backend", callback, notification);
                        self.es = Some(task);
                    }
                    EventSourceAction::Disconnect => {
                        self.es.take().unwrap().cancel();
                    }
                    EventSourceAction::Lost(status) => {
                        self.connection_state = status;
                        self.es = None;
                    }
                }
            }
            Msg::UpdateConnectionState(status) => {
                self.connection_state = status;
            }
            Msg::EsReady(response) => {
                match response {
                    Ok(data_result) => {
                        self.shared = Some(data_result);
                    }
                    Err(e) => {
                        let estr = format!("{}", e);
                        js!{ @(no_return) console.error("error in response", @{estr});}
                    }
                };
            }
            Msg::UpdateName(name) => {
                self.local_name = name;
            }
            Msg::SendName => {
                let name = self.local_name.clone();
                self.send_message("set_name", serde_json::value::to_value(name).unwrap(), context);
                return false; // don't update DOM, do that on return
            }
            Msg::ToggleRecording => {
                let new_value = if let Some(ref shared) = self.shared {
                    !shared.is_recording
                } else {
                    false
                };
                self.send_message("set_is_recording",
                    serde_json::value::to_value(new_value).unwrap(), context);
                return false; // don't update DOM, do that on return
            }
            Msg::Ignore => {
                return false;
            }
        }
        true
    }

}

impl Renderable<Context, Model> for Model {
    fn view(&self) -> Html<Context, Self> {
        html! {
            <div>
                { self.view_connection_state() }
                { self.view_shared() }
                { self.view_input() }
                <button onclick=|_| Msg::ToggleRecording,>{ "Toggle recording" }</button>
            </div>
        }
    }
}

impl Model {
    fn view_connection_state(&self) -> Html<Context, Model> {
        html! {
            <p>{ format!("Connection State: {:?}", self.connection_state) }</p>
        }

    }

    fn view_shared(&self) -> Html<Context, Model> {
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

    fn view_input(&self) -> Html<Context, Model> {
        html! {
            <input placeholder="name",
                   value=&self.local_name,
                   oninput=|e: InputData| Msg::UpdateName(e.value),
                   onblur=move|_| Msg::SendName,
                   onkeypress=|e: KeyData| {
                       if e.key == "Enter" { Msg::SendName } else { Msg::Ignore }
                   }, />
        }
    }

    fn send_message(&mut self, name: &str, args: serde_json::Value, context: &mut Env<Context, Self>) {
        let data = json!({
            "name": name,
            "args": args,
        });
        let buf = serde_json::to_string(&data).unwrap();
        let post_request = Request::post("callback")
                .header("Content-Type", "application/json;charset=UTF-8")
                .body(buf)
                .expect("Failed to build request.");
        let callback = context.send_back(|resp: Response<Result<String,failure::Error>>| {
            match resp.body() {
                &Ok(ref _s) => {}
                &Err(ref e) => {
                    let rs = format!("Error when sending message: {:?}", e);
                    js!{ @(no_return) console.error(@{rs})};
                }
            }
            Msg::Ignore
        });
        let task = context.web.fetch(post_request, callback, Some(&Credentials::SameOrigin));
        self.ft = Some(task);
    }
}

fn main() {
    yew::initialize();
    let context = Context {
        es: EventSourceService::new(),
        web: FetchService::new(),
    };
    let app: App<Context, Model> = App::new(context);
    app.mount_to_body();;
    yew::run_loop();
}
