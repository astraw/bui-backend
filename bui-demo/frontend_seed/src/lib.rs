#[macro_use]
extern crate seed;
use seed::prelude::*;
use seed::fetch;
use futures::Future;

use wasm_bindgen::JsCast;
use web_sys::{MessageEvent, EventSource};

use bui_demo_data::{Shared, Callback};

const ENTER_KEY: u32 = 13;

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
    Fetched(fetch::ResponseDataResult<()>),
}

fn update(msg: Msg, mut model: &mut Model, orders: &mut impl Orders<Msg>) {
    match msg {
        Msg::ServerMessage(msg_event) => {
            let txt = msg_event.data().as_string().unwrap();
            let response: Result<Shared,_> = serde_json::from_str(&txt);
            match response {
                Ok(data_result) => {
                    model.shared = Some(data_result);
                }
                Err(e) => {
                    error!("error in response", e)
                }
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
                .perform_cmd(send_message(&Callback::SetName(name)));
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
            orders
                .skip()
                .perform_cmd(send_message(&Callback::SetIsRecording(new_value)));
        }
        Msg::Fetched(Ok(_response_data)) => {
            // fetch successful
        }
        Msg::Fetched(Err(fail_reason)) => {
            error!("callback fetch error:", fail_reason);
            orders.skip();
        }
    }
}

fn send_message(payload: &Callback) -> impl Future<Output = Result<Msg,Msg>> {
    let url = "callback";
    fetch::Request::new(url)
        .method(fetch::Method::Post)
        .send_json(payload)
        .fetch_json_data(Msg::Fetched)

}

// -----------------------------------------------------------------------------

// View

fn view(model: &Model) -> Node<Msg> {
    div![
        h3!["BUI - Rust Backend, Seed Frontend - Demo"],
        p![
            button![
                simple_ev(Ev::Click, Msg::ToggleRecording),
                "Toggle Recording"
            ],
        ],
        p![
            label![
                "Name ",
                input![
                    attrs! {At::Value => &model.local_name},
                    simple_ev(Ev::Blur, Msg::SendName),
                    input_ev(Ev::Input, Msg::UpdateName),
                    keyboard_ev(Ev::KeyDown, move |ev| Msg::EditKeyDown(
                        ev.key_code()
                    )),
                ],
            ],
        ],
        p![
            h4!["backend state"],
            div![
                &format!("{:?}", model.shared),
            ]
        ],
        p![
            &format!("connection state: {}", model.connection_state),
        ],
    ]
}

// -----------------------------------------------------------------------------

fn after_mount(_: Url, orders: &mut impl Orders<Msg>) -> AfterMount<Model> {
    let events_url = "events";
    let es = EventSource::new(events_url).unwrap();
    let connection_state = es.ready_state();
    let shared = None;

    register_es_handler("open", Msg::Connected, &es, orders);
    register_es_handler("bui_backend", Msg::ServerMessage, &es, orders);
    register_es_handler("error", Msg::Error, &es, orders);

    AfterMount::new(Model {shared, es, local_name: "".to_string(), connection_state})
}

fn register_es_handler<T, F>(
    type_: &str,
    msg: F,
    es: &EventSource,
    orders: &mut impl Orders<Msg>,
) where
    T: wasm_bindgen::convert::FromWasmAbi + 'static,
    F: Fn(T) -> Msg + 'static,
{
    let (app, msg_mapper) = (orders.clone_app(), orders.msg_mapper());

    let closure = Closure::new(move |data| {
        app.update(msg_mapper(msg(data)));
    });

    es.add_event_listener_with_callback(type_, closure.as_ref().unchecked_ref()).unwrap();
    closure.forget();
}

#[wasm_bindgen(start)]
pub fn render() {
    seed::App::builder(update, view)
        .after_mount(after_mount)
        .build_and_start();
}
