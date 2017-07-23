function start() {
    var root = document.getElementById('root');
    var app = Elm.Main.embed(root);

    if (!!window.EventSource) {
        var source = new EventSource("events");

        source.addEventListener('message', function (e) {
            app.ports.event_source_data.send(e.data);
        }, false);

        source.addEventListener('open', function (e) {
            app.ports.event_source_connected.send(true);
        }, false);

        source.addEventListener('close', function (e) {
            app.ports.event_source_connected.send(false);
        }, false);

    } else {
        console.error("no EventSource. failing.");
    }
}

start();
