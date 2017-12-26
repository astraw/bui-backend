#[macro_use]
extern crate stdweb;
extern crate serde;
#[macro_use]
extern crate serde_json;
extern crate bui_demo_data;

use bui_demo_data::Shared;

use std::cell::RefCell;
use std::rc::Rc;

use stdweb::unstable::TryInto;
use stdweb::web::{IHtmlElement, IEventTarget, INode, document};
use stdweb::web::html_element::InputElement;

use stdweb::web::event::{IEvent, IKeyboardEvent, KeypressEvent, BlurEvent, ClickEvent};

// Shamelessly stolen from webplatform's TodoMVC example.
macro_rules! enclose {
    ( ($( $x:ident ),*) $y:expr ) => {
        {
            $(let $x = $x.clone();)*
            $y
        }
    };
}

type StateRef = Rc<RefCell<Option<Shared>>>;

fn send_message(name: &str, args: serde_json::Value) {
    let data = json!({
        "name": name,
        "args": args
    });
    let buf = serde_json::to_string(&data).unwrap();
    js!{
        var httpRequest = new XMLHttpRequest();
        httpRequest.open("POST", "callback");
        httpRequest.setRequestHeader("Content-Type", "application/json;charset=UTF-8");
        httpRequest.send(@{buf});
    }
}

fn update_dom(state: &StateRef) {
    // update mirror element (shows full serialized store)
    let mirror = document().get_element_by_id("mirror").unwrap();
    while let Some(child) = mirror.first_child() {
        mirror.remove_child(&child).unwrap();
    }

    let state_borrow = &*state.borrow();
    let text = match state_borrow {
        &Some(ref server_store) => serde_json::to_string(server_store).unwrap(),
        &None => "".to_string(),
    };

    let element = document().create_element("pre");
    element.append_child(&document().create_text_node(&text));
    mirror.append_child(&element);

    if let &Some(ref server_store) = state_borrow {
        // update the `is_recording` switch and progressbar
        js!{
            var my_switch = document.getElementById("switch-1-label").MaterialSwitch;
            var record_progress = document.getElementById("record-progress");
            if (@{server_store.is_recording}) {
                my_switch.on();
                record_progress.classList.add("mdl-progress__indeterminate");
            } else {
                my_switch.off();
                record_progress.classList.remove("mdl-progress__indeterminate");
            }

        }

        // update the `name` input field if it does not have focus
        js!{
            var my_textfield = document.getElementById("name-input-div");
            var has_focus = Boolean(my_textfield.querySelector(":focus"));
            if (!has_focus) {
                var name_input = document.getElementById("name-input");
                name_input.value = @{&server_store.name};
                my_textfield.MaterialTextfield.checkDirty();
            }
        }
    }

}

fn main() {
    stdweb::initialize();

    let state = Rc::new(RefCell::new(None));

    let on_message = enclose!( (state) move |buf: String| {
        // TODO: convert diretly from stdweb::Value without
        // another serialize/deserialize pass.
        match serde_json::from_str::<Shared>(&buf) {
            Ok(shared) => {
                *(state.borrow_mut()) = Some(shared);
                update_dom(&state);
            },
            Err(e) => {
                let errstr = format!("Error parsing Shared: {:?}", e);
                js!(console.error(@{errstr}););
            },
        }
    });

    let name_input: InputElement = document()
        .get_element_by_id("name-input")
        .unwrap()
        .try_into()
        .unwrap();
    name_input.add_event_listener(enclose!( (name_input) move |event: KeypressEvent| {
        if event.key() == "Enter" {
            event.prevent_default();
            let name: String = name_input.value().try_into().unwrap();
            send_message("set_name", serde_json::value::to_value(name).unwrap());
            name_input.blur();
        }
    }));
    name_input.add_event_listener(enclose!( (name_input) move |_: BlurEvent| {
        let name: String = name_input.value().try_into().unwrap();
        send_message("set_name", serde_json::value::to_value(name).unwrap());
    }));

    let recording_input: InputElement = document()
        .get_element_by_id("switch-1")
        .unwrap()
        .try_into()
        .unwrap();
    recording_input.add_event_listener(enclose!( (recording_input) move |_: ClickEvent| {
        let checked: bool = js!( return @{&recording_input}.checked; ).try_into().unwrap();
        send_message("set_is_recording", serde_json::value::to_value(checked).unwrap());
    }));

    let supports_event_source: bool = js!(return !!window.EventSource).try_into().unwrap();

    if supports_event_source {
        js! {
            var call_fn = @{on_message};
            var source = new EventSource("events");

            source.addEventListener("message", function (e) {
                var parsed = JSON.parse(e.data);
                call_fn(JSON.stringify(parsed.bui_backend));
            }, false);
        }
    } else {
        js!{
            var root = document.getElementById("root");
            root.innerHTML = ("<div>"+
                "<h4>EventSource not supported in this browser</h4>"+
                "Read about EventSource (also known as Server-sent events) at <a "+
                "href=\"https://html.spec.whatwg.org/multipage/"+
                "server-sent-events.html#server-sent-events\">whatwg.org</a>."+
                "See <a href=\"http://caniuse.com/#feat=eventsource\">caniuse.com</a> for "+
                "information about which browsers are supported."+
                "</div>");
        }
    }

    stdweb::event_loop();
}
