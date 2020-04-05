use crate::messages::Event;
use crate::parse_client_request::Timeline;

use std::{
    collections::{HashMap, VecDeque},
    fmt,
};
use uuid::Uuid;

#[derive(Clone)]
pub struct MsgQueue {
    pub timeline: Timeline,
    pub messages: VecDeque<Event>,
}

impl MsgQueue {
    pub fn new(timeline: Timeline) -> Self {
        MsgQueue {
            messages: VecDeque::new(),

            timeline,
        }
    }
}

#[derive(Debug)]
pub struct MessageQueues(pub HashMap<Uuid, MsgQueue>);

impl MessageQueues {}

impl fmt::Debug for MsgQueue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "\
MsgQueue {{
    timeline: {:?},
    messages: {:?},    
}}",
            self.timeline, self.messages,
        )
    }
}

impl std::ops::Deref for MessageQueues {
    type Target = HashMap<Uuid, MsgQueue>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
impl std::ops::DerefMut for MessageQueues {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}
