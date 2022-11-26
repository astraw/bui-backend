use std::{
    error::Error,
    fmt::{self, Debug, Display, Formatter},
};

use gloo_events::EventListener;
use wasm_bindgen::prelude::*;
use wasm_bindgen::{JsCast, JsValue};

use wasm_bindgen_futures::JsFuture;
use web_sys::{Event, EventSource, HtmlInputElement, MessageEvent};

use yew::events::KeyboardEvent;
use yew::prelude::*;

use bui_demo_data::{Callback, Shared};

pub struct App {
    es: EventSource,
    shared: Option<Shared>,
    local_name: String,
    _listener: EventListener,
}

pub enum FetchState {
    Fetching,
    Success,
    Failed(FetchError),
}

pub enum Msg {
    /// We got new data from the backend.
    EsReady(Result<Shared, serde_json::Error>),
    /// Update our local copy of our name. (E.g. the user typed a key.)
    UpdateName(String),
    /// We want to update name on the server. (E.g. the user pressed Enter.)
    SendName,
    /// We want to update the recording status on the server. (E.g. the user clicked the button.)
    ToggleRecording,
    SendMessageFetchState(FetchState),
    Ignore,
}

impl Component for App {
    type Message = Msg;
    type Properties = ();

    fn create(ctx: &Context<Self>) -> Self {
        let es = EventSource::new("events")
            .map_err(|js_value: JsValue| {
                let err: js_sys::Error = js_value.dyn_into().unwrap();
                err
            })
            .unwrap();

        let cb = ctx
            .link()
            .callback(|bufstr: String| Msg::EsReady(serde_json::from_str(&bufstr)));
        let listener = EventListener::new(&es, "bui_backend", move |event: &Event| {
            let event = event.dyn_ref::<MessageEvent>().unwrap();
            let text = event.data().as_string().unwrap();
            cb.emit(text);
        });

        Self {
            es,
            shared: None,
            local_name: "".to_string(),
            _listener: listener,
        }
    }

    fn update(&mut self, ctx: &Context<Self>, msg: Self::Message) -> bool {
        match msg {
            Msg::SendMessageFetchState(_fetch_state) => {
                // pass
            }
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
            Msg::UpdateName(name) => {
                self.local_name = name;
            }
            Msg::SendName => {
                let name = self.local_name.clone();

                ctx.link().send_future(async {
                    match post_callback(Callback::SetName(name)).await {
                        Ok(()) => Msg::SendMessageFetchState(FetchState::Success),
                        Err(err) => Msg::SendMessageFetchState(FetchState::Failed(err)),
                    }
                });
                ctx.link()
                    .send_message(Msg::SendMessageFetchState(FetchState::Fetching));
                return false; // Don't update DOM, do that when backend notifies us of new state.
            }
            Msg::ToggleRecording => {
                let new_value = if let Some(ref shared) = self.shared {
                    !shared.is_recording
                } else {
                    false
                };

                ctx.link().send_future(async move {
                    match post_callback(Callback::SetIsRecording(new_value)).await {
                        Ok(()) => Msg::SendMessageFetchState(FetchState::Success),
                        Err(err) => Msg::SendMessageFetchState(FetchState::Failed(err)),
                    }
                });
                ctx.link()
                    .send_message(Msg::SendMessageFetchState(FetchState::Fetching));
                return false; // Don't update DOM, do that when backend notifies us of new state.
            }
            Msg::Ignore => {
                return false;
            }
        }
        true
    }

    fn view(&self, ctx: &Context<Self>) -> Html {
        html! {
            <div>
                { self.view_ready_state() }
                { self.view_shared() }
                { self.view_input(ctx) }
                <button
                    onclick={ctx.link().callback(|_| Msg::ToggleRecording)}>
                    { "Toggle recording" }
                </button>
            </div>
        }
    }
}

impl App {
    fn view_ready_state(&self) -> Html {
        html! {
            <p>{ format!("Connection State: {:?}", self.es.ready_state()) }</p>
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

    fn view_input(&self, ctx: &Context<Self>) -> Html {
        html! {
            <input placeholder="name"
                   value={self.local_name.clone()}
                   oninput={ctx.link().callback(|e: InputEvent| {
                      let input: HtmlInputElement = e.target_unchecked_into();
                      Msg::UpdateName(input.value())
                   })}
                   onblur={ctx.link().callback(move|_| Msg::SendName)}
                   onkeypress={ctx.link().callback(|e: KeyboardEvent| {
                       if e.key() == "Enter" { Msg::SendName } else { Msg::Ignore }
                   })}
            />
        }
    }
}

/// Something wrong has occurred while fetching an external resource.
#[derive(Debug, Clone, PartialEq)]
pub struct FetchError {
    err: JsValue,
}
impl Display for FetchError {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        Debug::fmt(&self.err, f)
    }
}
impl Error for FetchError {}

impl From<JsValue> for FetchError {
    fn from(value: JsValue) -> Self {
        Self { err: value }
    }
}

async fn post_callback(msg: Callback) -> Result<(), FetchError> {
    use web_sys::{Request, RequestInit, Response};
    let mut opts = RequestInit::new();
    opts.method("POST");
    // opts.mode(web_sys::RequestMode::Cors);
    // opts.headers("Content-Type", "application/json;charset=UTF-8")
    // set SameOrigin
    let buf = serde_json::to_string(&msg).unwrap();
    opts.body(Some(&JsValue::from_str(&buf)));

    let url = "callback";
    let request = Request::new_with_str_and_init(url, &opts)?;

    let window = gloo_utils::window();
    let resp_value = JsFuture::from(window.fetch_with_request(&request)).await?;
    let resp: Response = resp_value.dyn_into().unwrap();

    let text = JsFuture::from(resp.text()?).await?;
    let _text_string = text.as_string().unwrap();
    Ok(())
}

#[wasm_bindgen(start)]
pub fn run_app() {
    wasm_logger::init(wasm_logger::Config::default());
    yew::start_app::<App>();
}
