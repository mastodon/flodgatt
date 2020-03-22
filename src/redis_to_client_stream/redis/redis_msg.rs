use crate::messages::Event;

#[derive(Debug)]
pub struct RedisMsg<'a> {
    pub raw: &'a str,
    pub cursor: usize,
    pub prefix_len: usize,
}

impl<'a> RedisMsg<'a> {
    pub fn from_raw(raw: &'a str, prefix_len: usize) -> Self {
        Self {
            raw,
            cursor: "*3\r\n".len(), //length of intro header
            prefix_len,
        }
    }
    /// Move the cursor from the beginning of a number through its end and return the number
    pub fn process_number(&mut self) -> usize {
        let (mut selected_number, selection_start) = (0, self.cursor);
        while let Ok(number) = self.raw[selection_start..=self.cursor].parse::<usize>() {
            self.cursor += 1;
            selected_number = number;
        }
        selected_number
    }
    /// In a pubsub reply from Redis, an item can be either the name of the subscribed channel
    /// or the msg payload.  Either way, it follows the same format:
    /// `$[LENGTH_OF_ITEM_BODY]\r\n[ITEM_BODY]\r\n`
    pub fn next_field(&mut self) -> String {
        self.cursor += "$".len();

        let item_len = self.process_number();
        self.cursor += "\r\n".len();
        let item_start_position = self.cursor;
        self.cursor += item_len;
        let item = self.raw[item_start_position..self.cursor].to_string();
        self.cursor += "\r\n".len();
        item
    }

    pub fn extract_raw_timeline_and_message(&mut self) -> (String, Event) {
        let timeline = &self.next_field()[self.prefix_len..];
        let msg_txt = self.next_field();
        let msg_value: Event =
            serde_json::from_str(&msg_txt).expect("Invariant violation: Invalid JSON from Redis");
        (timeline.to_string(), msg_value)
    }
}
