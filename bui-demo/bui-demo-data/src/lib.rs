#[macro_use]
extern crate serde_derive;

#[derive(Debug, PartialEq, Clone, Serialize, Deserialize)]
pub struct Shared {
    pub is_recording: bool,
    pub counter: usize,
    pub name: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum Callback {
    SetIsRecording(bool),
    SetName(String),
}
