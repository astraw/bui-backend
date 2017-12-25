var sever_event_obj = {
    onopen: function() {
        this._set_mirror("connecting");
    },
    onclose: function() {
        this._set_mirror("not connected");
    },
    onmessage: function(msg) {
        got_update(msg);
    },
    _set_mirror: function(text) {
        var mirror = document.getElementById("mirror");
        var content = document.createTextNode(text);

        while (mirror.firstChild) {
            mirror.removeChild(mirror.firstChild);
        }

        mirror.appendChild(content);
    }
}

function got_update(server_store){
    var mirror = document.getElementById("mirror");
    var buf = JSON.stringify(server_store);
    var element = document.createElement("pre");
    var content = document.createTextNode(buf);

    while (mirror.firstChild) {
        mirror.removeChild(mirror.firstChild);
    }

    element.appendChild(content);
    mirror.appendChild(element);

    var toggle = document.getElementById("switch-1");
    if (toggle.checked !== server_store.is_recording) {
        var my_switch = document.getElementById("switch-1-label").MaterialSwitch;
        if (server_store.is_recording) {
            my_switch.on();
        } else {
            my_switch.off();
        }
    }

    {
        var record_progress = document.getElementById("record-progress");
        if (server_store.is_recording) {
            record_progress.classList.add('mdl-progress__indeterminate');
        } else {
            record_progress.classList.remove('mdl-progress__indeterminate');
        }
    }

    var name_input = document.getElementById("name-input");
    if (name_input.value !== server_store.name) {
        var my_textfield = document.getElementById("name-input-div");
        var has_focus = Boolean(my_textfield.querySelector(':focus'));
        if (!has_focus) {
            name_input.value = server_store.name;
            my_textfield.MaterialTextfield.checkDirty();
        }
    }
}

function send_message(name,args){
    var msg = {
        name,
        args
    };
    var buf = JSON.stringify(msg);

    httpRequest = new XMLHttpRequest();
    httpRequest.open('POST', 'callback');
    httpRequest.setRequestHeader("Content-Type", "application/json;charset=UTF-8");
    httpRequest.send(buf);
}

document.getElementById("switch-1").onclick = function(event) {
    send_message("set_is_recording", event.target.checked);
 };

document.getElementById("name-input").addEventListener('blur',function(event) {
    send_message("set_name", event.target.value);
});

document.getElementById("name-input").addEventListener('keypress',function(event) {
    if (event.key == "Enter") {
        send_message("set_name", event.target.value);
        document.getElementById("name-input").blur();
    }
});

var SeverEvents = {
    init: function (sever_event_obj) {

        if (!!window.EventSource) {
            var source = new EventSource("events");

            source.addEventListener('message', function (e) {
                var parsed = JSON.parse(e.data);
                sever_event_obj.onmessage(parsed.bui_backend);
            }, false);

            source.addEventListener('open', function (e) {
                sever_event_obj.onopen()
            }, false);

            source.addEventListener('close', function (e) {
                sever_event_obj.onclose()
            }, false);


        } else {
            var root = document.getElementById("root");
            root.innerHTML = ('<div>'+
                '<h4>EventSource not supported in this browser</h4>'+
                'Read about EventSource (also known as Server-sent events) at <a '+
                'href="https://html.spec.whatwg.org/multipage/'+
                'server-sent-events.html#server-sent-events">whatwg.org</a>.'+
                'See <a href="http://caniuse.com/#feat=eventsource">caniuse.com</a> for '+
                'information about which browsers are supported.'+
                '</div>');
        }
    }
};

function start(){
    SeverEvents.init(sever_event_obj);
}

start();
