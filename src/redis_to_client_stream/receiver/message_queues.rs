use serde_json::Value;
use std::{collections, time};
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct MsgQueue {
    pub messages: collections::VecDeque<Value>,
    last_polled_at: time::Instant,
    pub redis_channel: String,
}

impl MsgQueue {
    pub fn new(redis_channel: impl std::fmt::Display) -> Self {
        let redis_channel = redis_channel.to_string();
        MsgQueue {
            messages: collections::VecDeque::new(),
            last_polled_at: time::Instant::now(),
            redis_channel,
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

    pub fn oldest_msg_in_target_queue(&mut self, id: Uuid, timeline: String) -> Option<Value> {
        self.entry(id)
            .or_insert_with(|| MsgQueue::new(timeline))
            .messages
            .pop_front()
    }
    pub fn calculate_timelines_to_add_or_drop(&mut self, timeline: String) -> Vec<Change> {
        let mut timelines_to_modify = Vec::new();

        timelines_to_modify.push(Change {
            timeline: timeline.to_owned(),
            in_subscriber_number: 1,
        });
        self.retain(|_id, msg_queue| {
            if msg_queue.last_polled_at.elapsed() < time::Duration::from_secs(30) {
                true
            } else {
                let timeline = &msg_queue.redis_channel;
                timelines_to_modify.push(Change {
                    timeline: timeline.to_owned(),
                    in_subscriber_number: -1,
                });
                false
            }
        });
        timelines_to_modify
    }
}
pub struct Change {
    pub timeline: String,
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
