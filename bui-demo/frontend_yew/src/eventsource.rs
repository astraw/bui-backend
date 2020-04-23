//! Event stream handling
use std::fmt;

use gloo::events::EventListener;
use wasm_bindgen::JsCast;
use web_sys::{Event, EventSource, MessageEvent};
use yew::callback::Callback;
use yew::format::{FormatError, Text};
use yew::services::Task;

/// A status of an event source connection. Used for status notification.
#[derive(PartialEq, Debug)]
pub enum EventSourceStatus {
    /// Fired when an event source connection was opened.
    Open,
    /// Fired when an event source connection had an error.
    Error,
}

#[derive(PartialEq, Debug)]
pub enum ReadyState {
    Connecting,
    Open,
    Closed,
}

/// A handle to control current event source connection. Implements `Task` and could be canceled.
pub struct EventSourceTask {
    event_source: EventSource,
    _notification: Callback<EventSourceStatus>,
    _listeners: [EventListener; 3],
}

impl EventSourceTask {
    fn new(
        event_source: EventSource,
        notification: Callback<EventSourceStatus>,
        listener_0: EventListener,
        listeners: [EventListener; 2],
    ) -> Result<EventSourceTask, &'static str> {
        let [listener_1, listener_2] = listeners;
        Ok(EventSourceTask {
            event_source,
            _notification: notification,
            _listeners: [listener_0, listener_1, listener_2],
        })
    }

    pub fn ready_state(&self) -> ReadyState {
        match self.event_source.ready_state() {
            web_sys::EventSource::CONNECTING => ReadyState::Connecting,
            web_sys::EventSource::OPEN => ReadyState::Open,
            web_sys::EventSource::CLOSED => ReadyState::Closed,
            _ => panic!("unexpected ready state"),
        }
    }
}

impl fmt::Debug for EventSourceTask {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("EventSourceTask")
    }
}

/// An event source service attached to a user context.
#[derive(Default, Debug)]
pub struct EventSourceService {}

impl EventSourceService {
    /// Creates a new service instance connected to `App` by provided `sender`.
    pub fn new() -> Self {
        Self {}
    }

    /// Connects to a server by an event source connection. Needs two functions to generate
    /// data and notification messages.
    pub fn connect<OUT: 'static, S>(
        &mut self,
        url: &str,
        event_type: S,
        callback: Callback<OUT>,
        notification: Callback<EventSourceStatus>,
    ) -> Result<EventSourceTask, &str>
    where
        S: Into<std::borrow::Cow<'static, str>>,
        OUT: From<Text>,
    {
        let event_source = EventSource::new(url);
        if event_source.is_err() {
            return Err("Failed to created event source with given URL");
        }

        let event_source = event_source.map_err(|_| "failed to build event source")?;

        let notify = notification.clone();
        let listener_open = move |_: &Event| {
            notify.emit(EventSourceStatus::Open);
        };
        let notify = notification.clone();
        let listener_error = move |_: &Event| {
            notify.emit(EventSourceStatus::Error);
        };

        let listeners = [
            EventListener::new(&event_source, "open", listener_open),
            EventListener::new(&event_source, "error", listener_error),
        ];

        let listener = EventListener::new(&event_source, event_type, move |event: &Event| {
            let event = event.dyn_ref::<MessageEvent>().unwrap();
            let text = event.data().as_string();

            let data = if let Some(text) = text {
                Ok(text)
            } else {
                Err(FormatError::ReceivedBinaryForText.into())
            };

            let out = OUT::from(data);
            callback.emit(out);
        });
        EventSourceTask::new(event_source, notification, listener, listeners)
    }
}

impl Task for EventSourceTask {
    fn is_active(&self) -> bool {
        self.ready_state() == ReadyState::Open
    }
}

impl Drop for EventSourceTask {
    fn drop(&mut self) {
        if self.is_active() {
            self.event_source.close()
        }
    }
}
