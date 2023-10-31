function update_dom(state) {
    var mirror = document.getElementById("mirror");
    var buf = JSON.stringify(state.server_store);
    if (state.ready_state != EventSource.OPEN) {
        // connection not in OPEN state
        buf = "connection state: " + state.ready_state;
    }
    var element = document.createElement("pre");
    var content = document.createTextNode(buf);

    while (mirror.firstChild) {
        mirror.removeChild(mirror.firstChild);
    }

    element.appendChild(content);
    mirror.appendChild(element);

    var toggle = document.getElementById("toggle-recording-button");
    toggle.onclick = function (event) {
        send_message({ SetIsRecording: !state.server_store.is_recording });
    };

    var name_input = document.getElementById("name-input");
    if (name_input.value !== state.server_store.name) {
        var my_textfield = document.getElementById("name-input-div");
        var has_focus = Boolean(my_textfield.querySelector(':focus'));
        if (!has_focus) {
            name_input.value = state.server_store.name;
        }
    }
}

function send_message(msg) {
    var buf = JSON.stringify(msg);

    var httpRequest = new XMLHttpRequest();
    httpRequest.open('POST', 'callback');
    httpRequest.setRequestHeader("Content-Type", "application/json;charset=UTF-8");
    httpRequest.setRequestHeader('Cache-Control', 'no-cache, no-store, max-age=0');
    httpRequest.send(buf);
}


document.getElementById("name-input").addEventListener('blur', function (event) {
    send_message({ SetName: event.target.value });
});

document.getElementById("name-input").addEventListener('keypress', function (event) {
    if (event.key == "Enter") {
        send_message({ SetName: event.target.value });
        document.getElementById("name-input").blur();
    }
});

var state = { ready_state: 0, server_store: {} };

var SeverEvents = {
    init: function () {

        if (!!window.EventSource) {
            var source = new EventSource("events");
            state.ready_state = source.readyState;

            source.addEventListener('bui_backend', function (e) {
                state.server_store = JSON.parse(e.data);
                update_dom(state);
            }, false);

            source.addEventListener('open', function (e) {
                state.ready_state = source.readyState;
                update_dom(state);
            }, false);

            source.addEventListener('error', function (e) {
                state.ready_state = source.readyState;
                update_dom(state);
            }, false);

        } else {
            var root = document.getElementById("root");
            root.innerHTML = ('<div>' +
                '<h4>EventSource not supported in this browser</h4>' +
                'Read about EventSource (also known as Server-sent events) at <a ' +
                'href="https://html.spec.whatwg.org/multipage/' +
                'server-sent-events.html#server-sent-events">whatwg.org</a>.' +
                'See <a href="http://caniuse.com/#feat=eventsource">caniuse.com</a> for ' +
                'information about which browsers are supported.' +
                '</div>');
        }
    }
};

function start() {
    SeverEvents.init();
}

start();
