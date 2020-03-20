use crate::parse_client_request::subscription::Timeline;
use serde_json::Value;
use std::{collections, fmt, time};
use uuid::Uuid;

#[derive(Clone)]
pub struct MsgQueue {
    pub timeline: Timeline,
    pub messages: collections::VecDeque<Value>,
    last_polled_at: time::Instant,
}
impl fmt::Debug for MsgQueue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "\
MsgQueue {{
    timeline: {:?},
    messages: {:?},
    last_polled_at: {:?},
}}",
            self.timeline,
            self.messages,
            self.last_polled_at.elapsed(),
        )
    }
}

impl MsgQueue {
    pub fn new(timeline: Timeline) -> Self {
        MsgQueue {
            messages: collections::VecDeque::new(),
            last_polled_at: time::Instant::now(),
            timeline,
        }
    }
}

#[derive(Debug)]
pub struct MessageQueues(pub collections::HashMap<Uuid, MsgQueue>);

impl MessageQueues {
    pub fn update_time_for_target_queue(&mut self, id: Uuid) {
        self.entry(id)
            .and_modify(|queue| queue.last_polled_at = time::Instant::now());
    }

    pub fn oldest_msg_in_target_queue(&mut self, id: Uuid, timeline: Timeline) -> Option<Value> {
        self.entry(id)
            .or_insert_with(|| MsgQueue::new(timeline))
            .messages
            .pop_front()
    }
    pub fn calculate_timelines_to_add_or_drop(&mut self, timeline: Timeline) -> Vec<Change> {
        let mut timelines_to_modify = Vec::new();

        timelines_to_modify.push(Change {
            timeline,
            in_subscriber_number: 1,
        });
        self.retain(|_id, msg_queue| {
            if msg_queue.last_polled_at.elapsed() < time::Duration::from_secs(30) {
                true
            } else {
                let timeline = &msg_queue.timeline;
                timelines_to_modify.push(Change {
                    timeline: *timeline,
                    in_subscriber_number: -1,
                });
                false
            }
        });
        timelines_to_modify
    }
}
pub struct Change {
    pub timeline: Timeline,
    pub in_subscriber_number: i32,
}

impl std::ops::Deref for MessageQueues {
    type Target = collections::HashMap<Uuid, MsgQueue>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
impl std::ops::DerefMut for MessageQueues {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}
