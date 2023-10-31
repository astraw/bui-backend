use seed::prelude::*;
use wasm_bindgen::closure::Closure;

use wasm_bindgen::JsCast;
use wasm_bindgen_futures::JsFuture;
use web_sys::{EventSource, MessageEvent};

use bui_demo_data::{Callback, Shared};

const ENTER_KEY: u32 = 13;

// -----------------------------------------------------------------------------

// Init

fn init(_: Url, orders: &mut impl Orders<Msg>) -> Model {
    let events_url = "events";
    let es = EventSource::new(events_url).unwrap();
    let connection_state = es.ready_state();
    let shared = None;

    register_es_handler("open", Msg::Connected, &es, orders);
    register_es_handler("bui_backend", Msg::ServerMessage, &es, orders);
    register_es_handler("error", Msg::Error, &es, orders);

    Model {
        shared,
        es,
        local_name: "".to_string(),
        connection_state,
    }
}

// -----------------------------------------------------------------------------

// Model

struct Model {
    shared: Option<Shared>,
    es: EventSource,
    local_name: String,
    connection_state: u16,
}

// -----------------------------------------------------------------------------

// Update

#[derive(Clone)]
pub enum Msg {
    Connected(JsValue),
    ServerMessage(MessageEvent),
    Error(JsValue),
    SendName,
    UpdateName(String),
    EditKeyDown(u32), // keycode
    ToggleRecording,
    Fetched(Result<(), String>),
}

fn update(msg: Msg, mut model: &mut Model, orders: &mut impl Orders<Msg>) {
    match msg {
        Msg::ServerMessage(msg_event) => {
            let txt = msg_event.data().as_string().unwrap();
            let response: Result<Shared, _> = serde_json::from_str(&txt);
            match response {
                Ok(data_result) => {
                    model.shared = Some(data_result);
                }
                Err(e) => gloo_console::error!(format!("error in response: {e}")),
            }
        }
        Msg::Connected(_) => {
            model.connection_state = model.es.ready_state();
        }
        Msg::Error(_) => {
            model.connection_state = model.es.ready_state();
        }
        Msg::SendName => {
            let name = model.local_name.clone();
            orders
                .skip()
                .perform_cmd(async { send_message(Callback::SetName(name)).await });
        }
        Msg::UpdateName(local_name) => {
            model.local_name = local_name;
        }
        Msg::EditKeyDown(code) => {
            if code == ENTER_KEY {
                orders.send_msg(Msg::SendName);
            }
        }
        Msg::ToggleRecording => {
            let new_value = if let Some(ref shared) = model.shared {
                !shared.is_recording
            } else {
                false
            };
            orders.skip().perform_cmd(async move {
                send_message(Callback::SetIsRecording(new_value)).await
            });
        }
        Msg::Fetched(Ok(_response_data)) => {
            // fetch successful
        }
        Msg::Fetched(Err(fail_reason)) => {
            (gloo_console::error!(format!("callback fetch error: {fail_reason}")));
            orders.skip();
        }
    }
}

async fn send_message(msg: Callback) -> Msg {
    use web_sys::{Request, RequestInit, Response};
    let mut opts = RequestInit::new();
    opts.method("POST");
    opts.cache(web_sys::RequestCache::NoStore);
    let buf = serde_json::to_string(&msg).unwrap();
    opts.body(Some(&JsValue::from_str(&buf)));

    let url = "callback";
    let request = Request::new_with_str_and_init(url, &opts).unwrap();

    let window = gloo_utils::window();
    let resp_value = JsFuture::from(window.fetch_with_request(&request))
        .await
        .unwrap();
    let resp: Response = resp_value.dyn_into().unwrap();

    let text = JsFuture::from(resp.text().unwrap()).await.unwrap();
    let _text_string = text.as_string().unwrap();
    Msg::Fetched(Ok(()))
}

// -----------------------------------------------------------------------------

// View

fn view(model: &Model) -> Node<Msg> {
    use seed::*;
    div![
        h3!["BUI - Rust Backend, Seed Frontend - Demo"],
        p![button![
            ev(Ev::Click, |_| Msg::ToggleRecording),
            "Toggle Recording"
        ],],
        p![label![
            "Name ",
            input![
                attrs! {At::Value => &model.local_name},
                ev(Ev::Blur, |_| Msg::SendName),
                input_ev(Ev::Input, Msg::UpdateName),
                keyboard_ev(Ev::KeyDown, move |ev| Msg::EditKeyDown(ev.key_code())),
            ],
        ],],
        p![h4!["backend state"], div![&format!("{:?}", model.shared),]],
        p![&format!("connection state: {}", model.connection_state),],
    ]
}

// -----------------------------------------------------------------------------

fn register_es_handler<T, F>(type_: &str, msg: F, es: &EventSource, orders: &mut impl Orders<Msg>)
where
    T: wasm_bindgen::convert::FromWasmAbi + 'static,
    F: Fn(T) -> Msg + 'static,
{
    let (app, msg_mapper) = (orders.clone_app(), orders.msg_mapper());

    let closure: Closure<dyn FnMut(T)> = Closure::new(move |data| {
        app.update(msg_mapper(msg(data)));
    });

    es.add_event_listener_with_callback(type_, closure.as_ref().unchecked_ref())
        .unwrap();
    closure.forget();
}

#[wasm_bindgen(start)]
pub fn start() {
    // Mount the `app` to the element with the `id` "app".
    App::start("app", init, update, view);
}
