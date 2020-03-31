// TODO: Revise these tests to cover *only* the RedisMessage -> (Timeline, Event) parsing
//     use super::*;
//     use crate::{
//         err::RedisParseErr,
//         parse_client_request::{Content::*, Reach::*, Stream::*, Timeline},
//         redis_to_client_stream::*,
//     };
//     use lru::LruCache;
//     use std::collections::HashMap;
//     use uuid::Uuid;
//     type Err = RedisParseErr;

//     /// Set up state shared between multiple tests of Redis parsing
//     pub fn shared_setup() -> (LruCache<String, i64>, MessageQueues, Uuid, Timeline) {
//         let mut cache: LruCache<String, i64> = LruCache::new(1000);
//         let mut queues_map = HashMap::new();
//         let id = dbg!(Uuid::default());

//         let timeline = dbg!(
//             Timeline::from_redis_raw_timeline("timeline:4", &mut cache, &None).expect("In test")
//         );
//         queues_map.insert(id, MsgQueue::new(timeline));
//         let queues = MessageQueues(queues_map);
//         (cache, queues, id, timeline)
//     }

//     #[test]
//     fn accurately_parse_redis_output_into_event() -> Result<(), Err> {
//         let input ="*3\r\n$7\r\nmessage\r\n$10\r\ntimeline:4\r\n$1386\r\n{\"event\":\"update\",\"payload\":{\"id\":\"102866835379605039\",\"created_at\":\"2019-09-27T22:29:02.590Z\",\"in_reply_to_id\":null,\"in_reply_to_account_id\":null,\"sensitive\":false,\"spoiler_text\":\"\",\"visibility\":\"public\",\"language\":\"en\",\"uri\":\"http://localhost:3000/users/admin/statuses/102866835379605039\",\"url\":\"http://localhost:3000/@admin/102866835379605039\",\"replies_count\":0,\"reblogs_count\":0,\"favourites_count\":0,\"favourited\":false,\"reblogged\":false,\"muted\":false,\"content\":\"<p><span class=\\\"h-card\\\"><a href=\\\"http://localhost:3000/@susan\\\" class=\\\"u-url mention\\\">@<span>susan</span></a></span> hi</p>\",\"reblog\":null,\"application\":{\"name\":\"Web\",\"website\":null},\"account\":{\"id\":\"1\",\"username\":\"admin\",\"acct\":\"admin\",\"display_name\":\"\",\"locked\":false,\"bot\":false,\"created_at\":\"2019-07-04T00:21:05.890Z\",\"note\":\"<p></p>\",\"url\":\"http://localhost:3000/@admin\",\"avatar\":\"http://localhost:3000/avatars/original/missing.png\",\"avatar_static\":\"http://localhost:3000/avatars/original/missing.png\",\"header\":\"http://localhost:3000/headers/original/missing.png\",\"header_static\":\"http://localhost:3000/headers/original/missing.png\",\"followers_count\":3,\"following_count\":3,\"statuses_count\":192,\"emojis\":[],\"fields\":[]},\"media_attachments\":[],\"mentions\":[{\"id\":\"4\",\"username\":\"susan\",\"url\":\"http://localhost:3000/@susan\",\"acct\":\"susan\"}],\"tags\":[],\"emojis\":[],\"card\":null,\"poll\":null},\"queued_at\":1569623342825}\r\n";

//         let (mut cache, mut queues, id, timeline) = shared_setup();
//         crate::redis_to_client_stream::process_msg(input, &mut cache, &mut None, &mut queues);

//         let parsed_event = queues.oldest_msg_in_target_queue(id, timeline).unwrap();
//         let test_event = Event::Update{ payload: Status {
//             id: "102866835379605039".to_string(),
//             created_at: "2019-09-27T22:29:02.590Z".to_string(),
//             in_reply_to_id: None,
//             in_reply_to_account_id: None,
//             sensitive: false,
//             spoiler_text: "".to_string(),
//             visibility: Visibility::Public,
//             language: Some("en".to_string()),
//             uri: "http://localhost:3000/users/admin/statuses/102866835379605039".to_string(),
//             url: Some("http://localhost:3000/@admin/102866835379605039".to_string()),
//             replies_count: 0,
//             reblogs_count: 0,
//             favourites_count: 0,
//             favourited: Some(false),
//             reblogged: Some(false),
//             muted: Some(false),
//             bookmarked: None,
//             pinned: None,
//             content: "<p><span class=\"h-card\"><a href=\"http://localhost:3000/@susan\" class=\"u-url mention\">@<span>susan</span></a></span> hi</p>".to_string(),
//             reblog: None,
//             application: Some(Application {
//                 name: "Web".to_string(),
//                 website: None,
//                 vapid_key: None,
//                 client_id: None,
//                 client_secret: None,
//             }),
//             account: Account {
//                 id: "1".to_string(),
//                 username: "admin".to_string(),
//                 acct: "admin".to_string(),
//                 display_name: "".to_string(),
//                 locked:false,
//                 bot:Some(false),
//                 created_at: "2019-07-04T00:21:05.890Z".to_string(),
//                 note:"<p></p>".to_string(),
//                 url:"http://localhost:3000/@admin".to_string(),
//                 avatar: "http://localhost:3000/avatars/original/missing.png".to_string(),
//                 avatar_static:"http://localhost:3000/avatars/original/missing.png".to_string(),
//                 header: "http://localhost:3000/headers/original/missing.png".to_string(),
//                 header_static:"http://localhost:3000/headers/original/missing.png".to_string(),
//                 followers_count:3,
//                 following_count:3,
//                 statuses_count:192,
//                 emojis:vec![],
//                 fields:Some(vec![]),
//                 moved: None,
//                 group: None,
//                 last_status_at: None,
//                 discoverable: None,
//                 source: None,
//             },
//             media_attachments:vec![],
//             mentions: vec![ Mention {id:"4".to_string(),
//                                      username:"susan".to_string(),
//                                      url:"http://localhost:3000/@susan".to_string(),
//                                      acct:"susan".to_string()}],
//             tags:vec![],
//             emojis:vec![],
//             card:None,poll:None,
//             text: None,
//         },
//         queued_at: Some(1569623342825)};

//         assert_eq!(parsed_event, test_event);
//         Ok(())
//     }

//     #[test]
//     fn parse_redis_input_subscription_msgs_and_update() -> Result<(), Err> {
//         let input = "*3\r\n$9\r\nsubscribe\r\n$11\r\ntimeline:56\r\n:1\r\n*3\r\n$9\r\nsubscribe\r\n$12\r\ntimeline:308\r\n:2\r\n*3\r\n$9\r\nsubscribe\r\n$21\r\ntimeline:hashtag:test\r\n:3\r\n*3\r\n$9\r\nsubscribe\r\n$21\r\ntimeline:public:local\r\n:4\r\n*3\r\n$9\r\nsubscribe\r\n$11\r\ntimeline:55\r\n:5\r\n*3\r\n$7\r\nmessage\r\n$21\r\ntimeline:public:local\r\n$1249\r\n{\"event\":\"update\",\"payload\":{\"id\":\"103881102123251272\",\"created_at\":\"2020-03-25T01:30:24.914Z\",\"in_reply_to_id\":null,\"in_reply_to_account_id\":null,\"sensitive\":false,\"spoiler_text\":\"\",\"visibility\":\"public\",\"language\":\"en\",\"uri\":\"https://instance.codesections.com/users/bob/statuses/103881102123251272\",\"url\":\"https://instance.codesections.com/@bob/103881102123251272\",\"replies_count\":0,\"reblogs_count\":0,\"favourites_count\":0,\"content\":\"<p>0111</p>\",\"reblog\":null,\"application\":{\"name\":\"Web\",\"website\":null},\"account\":{\"id\":\"55\",\"username\":\"bob\",\"acct\":\"bob\",\"display_name\":\"\",\"locked\":false,\"bot\":false,\"discoverable\":null,\"group\":false,\"created_at\":\"2020-03-11T03:03:53.068Z\",\"note\":\"<p></p>\",\"url\":\"https://instance.codesections.com/@bob\",\"avatar\":\"https://instance.codesections.com/avatars/original/missing.png\",\"avatar_static\":\"https://instance.codesections.com/avatars/original/missing.png\",\"header\":\"https://instance.codesections.com/headers/original/missing.png\",\"header_static\":\"https://instance.codesections.com/headers/original/missing.png\",\"followers_count\":1,\"following_count\":1,\"statuses_count\":57,\"last_status_at\":\"2020-03-25\",\"emojis\":[],\"fields\":[]},\"media_attachments\":[],\"mentions\":[],\"tags\":[],\"emojis\":[],\"card\":null,\"poll\":null}}\r\n*3\r\n$7\r\nmessage\r\n$11\r\ntimeline:55\r\n$1360\r\n{\"event\":\"update\",\"payload\":{\"id\":\"103881102123251272\",\"created_at\":\"2020-03-25T01:30:24.914Z\",\"in_reply_to_id\":null,\"in_reply_to_account_id\":null,\"sensitive\":false,\"spoiler_text\":\"\",\"visibility\":\"public\",\"language\":\"en\",\"uri\":\"https://instance.codesections.com/users/bob/statuses/103881102123251272\",\"url\":\"https://instance.codesections.com/@bob/103881102123251272\",\"replies_count\":0,\"reblogs_count\":0,\"favourites_count\":0,\"favourited\":false,\"reblogged\":false,\"muted\":false,\"bookmarked\":false,\"pinned\":false,\"content\":\"<p>0111</p>\",\"reblog\":null,\"application\":{\"name\":\"Web\",\"website\":null},\"account\":{\"id\":\"55\",\"username\":\"bob\",\"acct\":\"bob\",\"display_name\":\"\",\"locked\":false,\"bot\":false,\"discoverable\":null,\"group\":false,\"created_at\":\"2020-03-11T03:03:53.068Z\",\"note\":\"<p></p>\",\"url\":\"https://instance.codesections.com/@bob\",\"avatar\":\"https://instance.codesections.com/avatars/original/missing.png\",\"avatar_static\":\"https://instance.codesections.com/avatars/original/missing.png\",\"header\":\"https://instance.codesections.com/headers/original/missing.png\",\"header_static\":\"https://instance.codesections.com/headers/original/missing.png\",\"followers_count\":1,\"following_count\":1,\"statuses_count\":57,\"last_status_at\":\"2020-03-25\",\"emojis\":[],\"fields\":[]},\"media_attachments\":[],\"mentions\":[],\"tags\":[],\"emojis\":[],\"card\":null,\"poll\":null},\"queued_at\":1585099825263}\r\n*3\r\n$7\r\nmessage\r\n$21\r\ntimeline:public:local\r\n$1249\r\n{\"event\":\"update\",\"payload\":{\"id\":\"103881103451006570\",\"created_at\":\"2020-03-25T01:30:45.152Z\",\"in_reply_to_id\":null,\"in_reply_to_account_id\":null,\"sensitive\":false,\"spoiler_text\":\"\",\"visibility\":\"public\",\"language\":\"en\",\"uri\":\"https://instance.codesections.com/users/bob/statuses/103881103451006570\",\"url\":\"https://instance.codesections.com/@bob/103881103451006570\",\"replies_count\":0,\"reblogs_count\":0,\"favourites_count\":0,\"content\":\"<p>1000</p>\",\"reblog\":null,\"application\":{\"name\":\"Web\",\"website\":null},\"account\":{\"id\":\"55\",\"username\":\"bob\",\"acct\":\"bob\",\"display_name\":\"\",\"locked\":false,\"bot\":false,\"discoverable\":null,\"group\":false,\"created_at\":\"2020-03-11T03:03:53.068Z\",\"note\":\"<p></p>\",\"url\":\"https://instance.codesections.com/@bob\",\"avatar\":\"https://instance.codesections.com/avatars/original/missing.png\",\"avatar_static\":\"https://instance.codesections.com/avatars/original/missing.png\",\"header\":\"https://instance.codesections.com/headers/original/missing.png\",\"header_static\":\"https://instance.codesections.com/headers/original/missing.png\",\"followers_count\":1,\"following_count\":1,\"statuses_count\":58,\"last_status_at\":\"2020-03-25\",\"emojis\":[],\"fields\":[]},\"media_attachments\":[],\"mentions\":[],\"tags\":[],\"emojis\":[],\"card\":null,\"poll\":null}}\r\n*3\r\n$7\r\nmessage\r\n$11\r\ntimeline:55\r\n$1360\r\n{\"event\":\"update\",\"payload\":{\"id\":\"103881103451006570\",\"created_at\":\"2020-03-25T01:30:45.152Z\",\"in_reply_to_id\":null,\"in_reply_to_account_id\":null,\"sensitive\":false,\"spoiler_text\":\"\",\"visibility\":\"public\",\"language\":\"en\",\"uri\":\"https://instance.codesections.com/users/bob/statuses/103881103451006570\",\"url\":\"https://instance.codesections.com/@bob/103881103451006570\",\"replies_count\":0,\"reblogs_count\":0,\"favourites_count\":0,\"favourited\":false,\"reblogged\":false,\"muted\":false,\"bookmarked\":false,\"pinned\":false,\"content\":\"<p>1000</p>\",\"reblog\":null,\"application\":{\"name\":\"Web\",\"website\":null},\"account\":{\"id\":\"55\",\"username\":\"bob\",\"acct\":\"bob\",\"display_name\":\"\",\"locked\":false,\"bot\":false,\"discoverable\":null,\"group\":false,\"created_at\":\"2020-03-11T03:03:53.068Z\",\"note\":\"<p></p>\",\"url\":\"https://instance.codesections.com/@bob\",\"avatar\":\"https://instance.codesections.com/avatars/original/missing.png\",\"avatar_static\":\"https://instance.codesections.com/avatars/original/missing.png\",\"header\":\"https://instance.codesections.com/headers/original/missing.png\",\"header_static\":\"https://instance.codesections.com/headers/original/missing.png\",\"followers_count\":1,\"following_count\":1,\"statuses_count\":58,\"last_status_at\":\"2020-03-25\",\"emojis\":[],\"fields\":[]},\"media_attachments\":[],\"mentions\":[],\"tags\":[],\"emojis\":[],\"card\":null,\"poll\":null},\"queued_at\":1585099845405}\r\n";

//         let (mut cache, _, _, _) = shared_setup();

//         let (subscription_msg1, rest) = RedisMsg::from_raw(input, &mut cache, &None)?;
//         assert!(matches!(subscription_msg1, RedisMsg::SubscriptionMsg));

//         let (subscription_msg2, rest) = RedisMsg::from_raw(rest, &mut cache, &None)?;
//         assert!(matches!(subscription_msg2, RedisMsg::SubscriptionMsg));

//         let (subscription_msg3, rest) = RedisMsg::from_raw(rest, &mut cache, &None)?;
//         assert!(matches!(subscription_msg3, RedisMsg::SubscriptionMsg));

//         let (subscription_msg4, rest) = RedisMsg::from_raw(rest, &mut cache, &None)?;
//         assert!(matches!(subscription_msg4, RedisMsg::SubscriptionMsg));

//         let (subscription_msg5, rest) = RedisMsg::from_raw(rest, &mut cache, &None)?;
//         assert!(matches!(subscription_msg5, RedisMsg::SubscriptionMsg));

//         let (update_msg1, rest) = RedisMsg::from_raw(rest, &mut cache, &None)?;
//         assert!(matches!(
//             update_msg1,
//             RedisMsg::EventMsg(_, Event::Update { .. })
//         ));

//         let (update_msg2, rest) = RedisMsg::from_raw(rest, &mut cache, &None)?;
//         assert!(matches!(
//             update_msg2,
//             RedisMsg::EventMsg(_, Event::Update { .. })
//         ));

//         let (update_msg3, rest) = RedisMsg::from_raw(rest, &mut cache, &None)?;
//         assert!(matches!(
//             update_msg3,
//             RedisMsg::EventMsg(_, Event::Update { .. })
//         ));

//         let (update_msg4, rest) = RedisMsg::from_raw(rest, &mut cache, &None)?;
//         assert!(matches!(
//             update_msg4,
//             RedisMsg::EventMsg(_, Event::Update { .. })
//         ));

//         assert_eq!(rest, "".to_string());

//         Ok(())
//     }

//     #[test]
//     fn parse_redis_input_notification() -> Result<(), Err> {
//         let input = "*3\r\n$7\r\nmessage\r\n$11\r\ntimeline:55\r\n$2311\r\n{\"event\":\"notification\",\"payload\":{\"id\":\"147\",\"type\":\"mention\",\"created_at\":\"2020-03-25T14:25:09.295Z\",\"account\":{\"id\":\"308\",\"username\":\"ralph\",\"acct\":\"ralph\",\"display_name\":\"\",\"locked\":false,\"bot\":false,\"discoverable\":null,\"group\":false,\"created_at\":\"2020-03-11T19:55:20.933Z\",\"note\":\"<p></p>\",\"url\":\"https://instance.codesections.com/@ralph\",\"avatar\":\"https://instance.codesections.com/avatars/original/missing.png\",\"avatar_static\":\"https://instance.codesections.com/avatars/original/missing.png\",\"header\":\"https://instance.codesections.com/headers/original/missing.png\",\"header_static\":\"https://instance.codesections.com/headers/original/missing.png\",\"followers_count\":1,\"following_count\":2,\"statuses_count\":100,\"last_status_at\":\"2020-03-25\",\"emojis\":[],\"fields\":[]},\"status\":{\"id\":\"103884148503208016\",\"created_at\":\"2020-03-25T14:25:08.995Z\",\"in_reply_to_id\":null,\"in_reply_to_account_id\":null,\"sensitive\":false,\"spoiler_text\":\"\",\"visibility\":\"public\",\"language\":\"en\",\"uri\":\"https://instance.codesections.com/users/ralph/statuses/103884148503208016\",\"url\":\"https://instance.codesections.com/@ralph/103884148503208016\",\"replies_count\":0,\"reblogs_count\":0,\"favourites_count\":0,\"favourited\":false,\"reblogged\":false,\"muted\":false,\"bookmarked\":false,\"content\":\"<p><span class=\\\"h-card\\\"><a href=\\\"https://instance.codesections.com/@bob\\\" class=\\\"u-url mention\\\">@<span>bob</span></a></span> notification test</p>\",\"reblog\":null,\"application\":{\"name\":\"Web\",\"website\":null},\"account\":{\"id\":\"308\",\"username\":\"ralph\",\"acct\":\"ralph\",\"display_name\":\"\",\"locked\":false,\"bot\":false,\"discoverable\":null,\"group\":false,\"created_at\":\"2020-03-11T19:55:20.933Z\",\"note\":\"<p></p>\",\"url\":\"https://instance.codesections.com/@ralph\",\"avatar\":\"https://instance.codesections.com/avatars/original/missing.png\",\"avatar_static\":\"https://instance.codesections.com/avatars/original/missing.png\",\"header\":\"https://instance.codesections.com/headers/original/missing.png\",\"header_static\":\"https://instance.codesections.com/headers/original/missing.png\",\"followers_count\":1,\"following_count\":2,\"statuses_count\":100,\"last_status_at\":\"2020-03-25\",\"emojis\":[],\"fields\":[]},\"media_attachments\":[],\"mentions\":[{\"id\":\"55\",\"username\":\"bob\",\"url\":\"https://instance.codesections.com/@bob\",\"acct\":\"bob\"}],\"tags\":[],\"emojis\":[],\"card\":null,\"poll\":null}}}\r\n";

//         let (mut cache, _, _, _) = shared_setup();

//         let (subscription_msg1, rest) = RedisMsg::from_raw(input, &mut cache, &None)?;
//         assert!(matches!(
//             subscription_msg1,
//             RedisMsg::EventMsg(Timeline(User(id), Federated, All), Event::Notification { .. }) if id == 55
//         ));

//         assert_eq!(rest, "".to_string());

//         Ok(())
//     }

//     #[test]
//     fn parse_redis_input_delete() -> Result<(), Err> {
//         let input = "*3\r\n$7\r\nmessage\r\n$12\r\ntimeline:308\r\n$49\r\n{\"event\":\"delete\",\"payload\":\"103864778284581232\"}\r\n";

//         let (mut cache, _, _, _) = dbg!(shared_setup());

//         let (subscription_msg1, rest) = RedisMsg::from_raw(input, &mut cache, &None)?;
//         assert!(matches!(
//             subscription_msg1,
//             RedisMsg::EventMsg(
//                 Timeline(User(308), Federated, All),
//                 Event::Delete { payload: DeletedId(id) }
//             ) if id == "103864778284581232".to_string()
//         ));

//         assert_eq!(rest, "".to_string());

//         Ok(())
//     }

//     #[test]
//     fn parse_redis_input_filters_changed() -> Result<(), Err> {
//         let input = "*3\r\n$7\r\nmessage\r\n$11\r\ntimeline:56\r\n$27\r\n{\"event\":\"filters_changed\"}\r\n";

//         let (mut cache, _, _, _) = shared_setup();

//         let (subscription_msg1, rest) = RedisMsg::from_raw(input, &mut cache, &None)?;
//         assert!(matches!(
//             subscription_msg1,
//             RedisMsg::EventMsg(Timeline(User(id), Federated, All), Event::FiltersChanged) if id == 56
//         ));

//         assert_eq!(rest, "".to_string());

//         Ok(())
//     }

//     #[test]
//     fn parse_redis_input_announcement() -> Result<(), Err> {
//         let input = "*3\r\n$7\r\nmessage\r\n$12\r\ntimeline:308\r\n$293\r\n{\"event\":\"announcement\",\"payload\":{\"id\":\"2\",\"content\":\"<p>Test announcement 0010</p>\",\"starts_at\":null,\"ends_at\":null,\"all_day\":false,\"published_at\":\"2020-03-25T14:57:57.550Z\",\"updated_at\":\"2020-03-25T14:57:57.566Z\",\"mentions\":[],\"tags\":[],\"emojis\":[],\"reactions\":[{\"name\":\"👍\",\"count\":2}]}}\r\n";

//         let (mut cache, _, _, _) = shared_setup();

//         let (msg, rest) = RedisMsg::from_raw(input, &mut cache, &None)?;
//         assert!(matches!(
//             msg,
//             RedisMsg::EventMsg(
//                 Timeline(User(id), Federated, All),
//                 Event::Announcement { .. }) if id == 308
//         ));

//         assert_eq!(rest, "".to_string());
//         Ok(())
//     }

//     #[test]
//     fn parse_redis_input_announcement_reaction() -> Result<(), Err> {
//         let input = "*3\r\n$7\r\nmessage\r\n$12\r\ntimeline:308\r\n$91\r\n{\"event\":\"announcement.reaction\",\"payload\":{\"name\":\"👽\",\"count\":2,\"announcement_id\":\"8\"}}\r\n";

//         let (mut cache, _, _, _) = shared_setup();

//         let (msg, rest) = RedisMsg::from_raw(input, &mut cache, &None)?;
//         assert!(matches!(
//             msg,
//             RedisMsg::EventMsg(
//                 Timeline(User(id), Federated, All),
//                 Event::AnnouncementReaction{ .. }
//             ) if id == 308
//         ));

//         assert_eq!(rest, "".to_string());
//         Ok(())
//     }

//     #[test]
//     fn parse_redis_input_announcement_delete() -> Result<(), Err> {
//         let input = "*3\r\n$7\r\nmessage\r\n$12\r\ntimeline:308\r\n$45\r\n{\"event\":\"announcement.delete\",\"payload\":\"5\"}\r\n";

//         let (mut cache, _, _, _) = shared_setup();

//         let (msg, rest) = RedisMsg::from_raw(input, &mut cache, &None)?;
//         assert!(matches!(
//             msg,
//             RedisMsg::EventMsg(
//                 Timeline(User(id), Federated, All),
//                 Event::AnnouncementDelete{
//                     payload: DeletedId(del_id),

//                 }
//             ) if id == 308 && del_id == "5".to_string()
//         ));

//         assert_eq!(rest, "".to_string());
//         Ok(())
//     }

//     #[test]
//     fn parse_redis_input_status_with_attachments() -> Result<(), Err> {
//         let input = "*3\r\n$7\r\nmessage\r\n$12\r\ntimeline:308\r\n$2049\r\n{\"event\":\"update\",\"payload\":{\"id\":\"103884996729070829\",\"created_at\":\"2020-03-25T18:00:52.026Z\",\"in_reply_to_id\":null,\"in_reply_to_account_id\":null,\"sensitive\":false,\"spoiler_text\":\"\",\"visibility\":\"public\",\"language\":\"en\",\"uri\":\"https://instance.codesections.com/users/ralph/statuses/103884996729070829\",\"url\":\"https://instance.codesections.com/@ralph/103884996729070829\",\"replies_count\":0,\"reblogs_count\":0,\"favourites_count\":0,\"favourited\":false,\"reblogged\":false,\"muted\":false,\"bookmarked\":false,\"pinned\":false,\"content\":\"<p>Test with media attachment</p>\",\"reblog\":null,\"application\":{\"name\":\"Web\",\"website\":null},\"account\":{\"id\":\"308\",\"username\":\"ralph\",\"acct\":\"ralph\",\"display_name\":\"\",\"locked\":false,\"bot\":false,\"discoverable\":null,\"group\":false,\"created_at\":\"2020-03-11T19:55:20.933Z\",\"note\":\"<p></p>\",\"url\":\"https://instance.codesections.com/@ralph\",\"avatar\":\"https://instance.codesections.com/avatars/original/missing.png\",\"avatar_static\":\"https://instance.codesections.com/avatars/original/missing.png\",\"header\":\"https://instance.codesections.com/headers/original/missing.png\",\"header_static\":\"https://instance.codesections.com/headers/original/missing.png\",\"followers_count\":1,\"following_count\":2,\"statuses_count\":103,\"last_status_at\":\"2020-03-25\",\"emojis\":[],\"fields\":[]},\"media_attachments\":[{\"id\":\"3102\",\"type\":\"image\",\"url\":\"https://instance.codesections.com/system/media_attachments/files/000/003/102/original/1753cf5b8edd544a.jpg?1585159208\",\"preview_url\":\"https://instance.codesections.com/system/media_attachments/files/000/003/102/small/1753cf5b8edd544a.jpg?1585159208\",\"remote_url\":null,\"text_url\":\"https://instance.codesections.com/media/7XPfdkmAIHb3TQcLYII\",\"meta\":{\"original\":{\"width\":828,\"height\":340,\"size\":\"828x340\",\"aspect\":2.4352941176470586},\"small\":{\"width\":623,\"height\":256,\"size\":\"623x256\",\"aspect\":2.43359375},\"focus\":{\"x\":0.0,\"y\":0.0}},\"description\":\"Test image discription\",\"blurhash\":\"UBR{.4M{s;IU0JkBWBWB9bM{ofxu4^WAWBj[\"}],\"mentions\":[],\"tags\":[],\"emojis\":[],\"card\":null,\"poll\":null},\"queued_at\":1585159252656}\r\n";

//         let (mut cache, _, _, _) = shared_setup();

//         let (msg, rest) = RedisMsg::from_raw(input, &mut cache, &None)?;
//         dbg!(&msg);
//         assert!(matches!(
//             msg,
//             RedisMsg::EventMsg(
//                 Timeline(User(308), Federated, All),
//                 Event::Update{ payload: Status { media_attachments: attachments, .. }, ..  }
//             ) if attachments.len() > 0
//         ));

//         assert_eq!(rest, "".to_string());
//         Ok(())
//     }

//     #[test]
//     fn parse_redis_input_status_with_mentions() -> Result<(), Err> {
//         let input = "*3\r\n$7\r\nmessage\r\n$12\r\ntimeline:308\r\n$2094\r\n{\"event\":\"update\",\"payload\":{\"id\":\"103885034181231245\",\"created_at\":\"2020-03-25T18:10:23.420Z\",\"in_reply_to_id\":null,\"in_reply_to_account_id\":null,\"sensitive\":false,\"spoiler_text\":\"\",\"visibility\":\"public\",\"language\":\"en\",\"uri\":\"https://instance.codesections.com/users/ralph/statuses/103885034181231245\",\"url\":\"https://instance.codesections.com/@ralph/103885034181231245\",\"replies_count\":0,\"reblogs_count\":0,\"favourites_count\":0,\"favourited\":false,\"reblogged\":false,\"muted\":false,\"bookmarked\":false,\"pinned\":false,\"content\":\"<p><span class=\\\"h-card\\\"><a href=\\\"https://instance.codesections.com/@bob\\\" class=\\\"u-url mention\\\">@<span>bob</span></a></span> <span class=\\\"h-card\\\"><a href=\\\"https://instance.codesections.com/@susan\\\" class=\\\"u-url mention\\\">@<span>susan</span></a></span> <span class=\\\"h-card\\\"><a href=\\\"https://instance.codesections.com/@codesections\\\" class=\\\"u-url mention\\\">@<span>codesections</span></a></span> </p><p>Test with mentions</p>\",\"reblog\":null,\"application\":{\"name\":\"Web\",\"website\":null},\"account\":{\"id\":\"308\",\"username\":\"ralph\",\"acct\":\"ralph\",\"display_name\":\"\",\"locked\":false,\"bot\":false,\"discoverable\":null,\"group\":false,\"created_at\":\"2020-03-11T19:55:20.933Z\",\"note\":\"<p></p>\",\"url\":\"https://instance.codesections.com/@ralph\",\"avatar\":\"https://instance.codesections.com/avatars/original/missing.png\",\"avatar_static\":\"https://instance.codesections.com/avatars/original/missing.png\",\"header\":\"https://instance.codesections.com/headers/original/missing.png\",\"header_static\":\"https://instance.codesections.com/headers/original/missing.png\",\"followers_count\":1,\"following_count\":2,\"statuses_count\":104,\"last_status_at\":\"2020-03-25\",\"emojis\":[],\"fields\":[]},\"media_attachments\":[],\"mentions\":[{\"id\":\"55\",\"username\":\"bob\",\"url\":\"https://instance.codesections.com/@bob\",\"acct\":\"bob\"},{\"id\":\"56\",\"username\":\"susan\",\"url\":\"https://instance.codesections.com/@susan\",\"acct\":\"susan\"},{\"id\":\"9\",\"username\":\"codesections\",\"url\":\"https://instance.codesections.com/@codesections\",\"acct\":\"codesections\"}],\"tags\":[],\"emojis\":[],\"card\":null,\"poll\":null},\"queued_at\":1585159824540}\r\n";

//         let (mut cache, _, _, _) = shared_setup();

//         let (msg, rest) = RedisMsg::from_raw(input, &mut cache, &None)?;
//         dbg!(&msg);
//         assert!(matches!(
//             msg,
//             RedisMsg::EventMsg(
//                 Timeline(User(308), Federated, All),
//                 Event::Update{ payload: Status { mentions, .. }, ..  }
//             ) if mentions.len() > 0
//         ));

//         assert_eq!(rest, "".to_string());
//         Ok(())
//     }

//     #[test]
//     fn parse_redis_input_status_with_tags() -> Result<(), Err> {
//         let input = "*3\r\n$7\r\nmessage\r\n$12\r\ntimeline:308\r\n$1770\r\n{\"event\":\"update\",\"payload\":{\"id\":\"103885047114641861\",\"created_at\":\"2020-03-25T18:13:40.741Z\",\"in_reply_to_id\":null,\"in_reply_to_account_id\":null,\"sensitive\":false,\"spoiler_text\":\"\",\"visibility\":\"public\",\"language\":\"en\",\"uri\":\"https://instance.codesections.com/users/ralph/statuses/103885047114641861\",\"url\":\"https://instance.codesections.com/@ralph/103885047114641861\",\"replies_count\":0,\"reblogs_count\":0,\"favourites_count\":0,\"favourited\":false,\"reblogged\":false,\"muted\":false,\"bookmarked\":false,\"pinned\":false,\"content\":\"<p><a href=\\\"https://instance.codesections.com/tags/test\\\" class=\\\"mention hashtag\\\" rel=\\\"tag\\\">#<span>test</span></a> <a href=\\\"https://instance.codesections.com/tags/hashtag\\\" class=\\\"mention hashtag\\\" rel=\\\"tag\\\">#<span>hashtag</span></a> </p><p>Test with tags</p>\",\"reblog\":null,\"application\":{\"name\":\"Web\",\"website\":null},\"account\":{\"id\":\"308\",\"username\":\"ralph\",\"acct\":\"ralph\",\"display_name\":\"\",\"locked\":false,\"bot\":false,\"discoverable\":null,\"group\":false,\"created_at\":\"2020-03-11T19:55:20.933Z\",\"note\":\"<p></p>\",\"url\":\"https://instance.codesections.com/@ralph\",\"avatar\":\"https://instance.codesections.com/avatars/original/missing.png\",\"avatar_static\":\"https://instance.codesections.com/avatars/original/missing.png\",\"header\":\"https://instance.codesections.com/headers/original/missing.png\",\"header_static\":\"https://instance.codesections.com/headers/original/missing.png\",\"followers_count\":1,\"following_count\":2,\"statuses_count\":105,\"last_status_at\":\"2020-03-25\",\"emojis\":[],\"fields\":[]},\"media_attachments\":[],\"mentions\":[],\"tags\":[{\"name\":\"hashtag\",\"url\":\"https://instance.codesections.com/tags/hashtag\"},{\"name\":\"test\",\"url\":\"https://instance.codesections.com/tags/test\"}],\"emojis\":[],\"card\":null,\"poll\":null},\"queued_at\":1585160021281}\r\n";

//         let (mut cache, _, _, _) = shared_setup();

//         let (msg, rest) = RedisMsg::from_raw(input, &mut cache, &None)?;
//         dbg!(&msg);
//         assert!(matches!(
//             msg,
//                 RedisMsg::EventMsg(
//                     Timeline(User(308), Federated, All),
//                     Event::Update{ payload: Status { tags, .. }, ..  }
//                 ) if tags.len() > 0
//         ));

//         assert_eq!(rest, "".to_string());
//         Ok(())
//     }

//     #[test]
//     fn parse_redis_input_status_with_emojis() -> Result<(), Err> {
//         let input = "*3\r\n$7\r\nmessage\r\n$12\r\ntimeline:308\r\n$1703\r\n{\"event\":\"update\",\"payload\":{\"id\":\"103885068078872546\",\"created_at\":\"2020-03-25T18:19:00.620Z\",\"in_reply_to_id\":null,\"in_reply_to_account_id\":null,\"sensitive\":false,\"spoiler_text\":\"\",\"visibility\":\"public\",\"language\":\"en\",\"uri\":\"https://instance.codesections.com/users/ralph/statuses/103885068078872546\",\"url\":\"https://instance.codesections.com/@ralph/103885068078872546\",\"replies_count\":0,\"reblogs_count\":0,\"favourites_count\":0,\"favourited\":false,\"reblogged\":false,\"muted\":false,\"bookmarked\":false,\"pinned\":false,\"content\":\"<p>Test with custom emoji</p><p>:patcat:</p>\",\"reblog\":null,\"application\":{\"name\":\"Web\",\"website\":null},\"account\":{\"id\":\"308\",\"username\":\"ralph\",\"acct\":\"ralph\",\"display_name\":\"\",\"locked\":false,\"bot\":false,\"discoverable\":null,\"group\":false,\"created_at\":\"2020-03-11T19:55:20.933Z\",\"note\":\"<p></p>\",\"url\":\"https://instance.codesections.com/@ralph\",\"avatar\":\"https://instance.codesections.com/avatars/original/missing.png\",\"avatar_static\":\"https://instance.codesections.com/avatars/original/missing.png\",\"header\":\"https://instance.codesections.com/headers/original/missing.png\",\"header_static\":\"https://instance.codesections.com/headers/original/missing.png\",\"followers_count\":1,\"following_count\":2,\"statuses_count\":106,\"last_status_at\":\"2020-03-25\",\"emojis\":[],\"fields\":[]},\"media_attachments\":[],\"mentions\":[],\"tags\":[],\"emojis\":[{\"shortcode\":\"patcat\",\"url\":\"https://instance.codesections.com/system/custom_emojis/images/000/001/071/original/d87fcdf79ed6fe20.png?1585160295\",\"static_url\":\"https://instance.codesections.com/system/custom_emojis/images/000/001/071/static/d87fcdf79ed6fe20.png?1585160295\",\"visible_in_picker\":true}],\"card\":null,\"poll\":null},\"queued_at\":1585160340991}\r\n";

//         let (mut cache, _, _, _) = shared_setup();

//         let (msg, rest) = RedisMsg::from_raw(input, &mut cache, &None)?;
//         dbg!(&msg);
//         assert!(matches!(
//             msg,
//             RedisMsg::EventMsg(
//                 Timeline(User(308), Federated, All),
//                 Event::Update{ payload: Status { emojis, .. }, ..  }
//             ) if emojis.len() > 0
//         ));

//         assert_eq!(rest, "".to_string());
//         Ok(())
//     }

//     #[test]
//     fn parse_redis_input_status_is_reply() -> Result<(), Err> {
//         let input = "*3\r\n$7\r\nmessage\r\n$12\r\ntimeline:308\r\n$1612\r\n{\"event\":\"update\",\"payload\":{\"id\":\"103885083636011552\",\"created_at\":\"2020-03-25T18:22:57.963Z\",\"in_reply_to_id\":\"103881103451006570\",\"in_reply_to_account_id\":\"55\",\"sensitive\":false,\"spoiler_text\":\"\",\"visibility\":\"public\",\"language\":\"en\",\"uri\":\"https://instance.codesections.com/users/ralph/statuses/103885083636011552\",\"url\":\"https://instance.codesections.com/@ralph/103885083636011552\",\"replies_count\":0,\"reblogs_count\":0,\"favourites_count\":0,\"favourited\":false,\"reblogged\":false,\"muted\":false,\"bookmarked\":false,\"pinned\":false,\"content\":\"<p><span class=\\\"h-card\\\"><a href=\\\"https://instance.codesections.com/@bob\\\" class=\\\"u-url mention\\\">@<span>bob</span></a></span> Test is reply</p>\",\"reblog\":null,\"application\":{\"name\":\"Web\",\"website\":null},\"account\":{\"id\":\"308\",\"username\":\"ralph\",\"acct\":\"ralph\",\"display_name\":\"\",\"locked\":false,\"bot\":false,\"discoverable\":null,\"group\":false,\"created_at\":\"2020-03-11T19:55:20.933Z\",\"note\":\"<p></p>\",\"url\":\"https://instance.codesections.com/@ralph\",\"avatar\":\"https://instance.codesections.com/avatars/original/missing.png\",\"avatar_static\":\"https://instance.codesections.com/avatars/original/missing.png\",\"header\":\"https://instance.codesections.com/headers/original/missing.png\",\"header_static\":\"https://instance.codesections.com/headers/original/missing.png\",\"followers_count\":1,\"following_count\":2,\"statuses_count\":107,\"last_status_at\":\"2020-03-25\",\"emojis\":[],\"fields\":[]},\"media_attachments\":[],\"mentions\":[{\"id\":\"55\",\"username\":\"bob\",\"url\":\"https://instance.codesections.com/@bob\",\"acct\":\"bob\"}],\"tags\":[],\"emojis\":[],\"card\":null,\"poll\":null},\"queued_at\":1585160578486}\r\n*3\r\n$7\r\nmessage\r\n$11\r\ntimeline:55\r\n$2323\r\n{\"event\":\"notification\",\"payload\":{\"id\":\"156\",\"type\":\"mention\",\"created_at\":\"2020-03-25T18:22:58.293Z\",\"account\":{\"id\":\"308\",\"username\":\"ralph\",\"acct\":\"ralph\",\"display_name\":\"\",\"locked\":false,\"bot\":false,\"discoverable\":null,\"group\":false,\"created_at\":\"2020-03-11T19:55:20.933Z\",\"note\":\"<p></p>\",\"url\":\"https://instance.codesections.com/@ralph\",\"avatar\":\"https://instance.codesections.com/avatars/original/missing.png\",\"avatar_static\":\"https://instance.codesections.com/avatars/original/missing.png\",\"header\":\"https://instance.codesections.com/headers/original/missing.png\",\"header_static\":\"https://instance.codesections.com/headers/original/missing.png\",\"followers_count\":1,\"following_count\":2,\"statuses_count\":107,\"last_status_at\":\"2020-03-25\",\"emojis\":[],\"fields\":[]},\"status\":{\"id\":\"103885083636011552\",\"created_at\":\"2020-03-25T18:22:57.963Z\",\"in_reply_to_id\":\"103881103451006570\",\"in_reply_to_account_id\":\"55\",\"sensitive\":false,\"spoiler_text\":\"\",\"visibility\":\"public\",\"language\":\"en\",\"uri\":\"https://instance.codesections.com/users/ralph/statuses/103885083636011552\",\"url\":\"https://instance.codesections.com/@ralph/103885083636011552\",\"replies_count\":0,\"reblogs_count\":0,\"favourites_count\":0,\"favourited\":false,\"reblogged\":false,\"muted\":false,\"bookmarked\":false,\"content\":\"<p><span class=\\\"h-card\\\"><a href=\\\"https://instance.codesections.com/@bob\\\" class=\\\"u-url mention\\\">@<span>bob</span></a></span> Test is reply</p>\",\"reblog\":null,\"application\":{\"name\":\"Web\",\"website\":null},\"account\":{\"id\":\"308\",\"username\":\"ralph\",\"acct\":\"ralph\",\"display_name\":\"\",\"locked\":false,\"bot\":false,\"discoverable\":null,\"group\":false,\"created_at\":\"2020-03-11T19:55:20.933Z\",\"note\":\"<p></p>\",\"url\":\"https://instance.codesections.com/@ralph\",\"avatar\":\"https://instance.codesections.com/avatars/original/missing.png\",\"avatar_static\":\"https://instance.codesections.com/avatars/original/missing.png\",\"header\":\"https://instance.codesections.com/headers/original/missing.png\",\"header_static\":\"https://instance.codesections.com/headers/original/missing.png\",\"followers_count\":1,\"following_count\":2,\"statuses_count\":107,\"last_status_at\":\"2020-03-25\",\"emojis\":[],\"fields\":[]},\"media_attachments\":[],\"mentions\":[{\"id\":\"55\",\"username\":\"bob\",\"url\":\"https://instance.codesections.com/@bob\",\"acct\":\"bob\"}],\"tags\":[],\"emojis\":[],\"card\":null,\"poll\":null}}}\r\n";

//         let (mut cache, _, _, _) = shared_setup();

//         let (msg, rest) = RedisMsg::from_raw(input, &mut cache, &None)?;
//         dbg!(&msg);
//         assert!(matches!(
//             msg,
//             RedisMsg::EventMsg(
//                 Timeline(User(308), Federated, All),
//                 Event::Update {
//                     payload:
//                         Status {
//                             in_reply_to_id: Some(_),
//                             ..
//                         },
//                     ..
//                 },
//             )
//         ));
//         let (msg2, rest) = RedisMsg::from_raw(rest, &mut cache, &None)?;
//         dbg!(&msg2);
//         assert!(matches!(
//             msg2,
//             RedisMsg::EventMsg(Timeline(User(55), Federated, All), Event::Notification { .. })
//         ));

//         assert_eq!(rest, "".to_string());
//         Ok(())
//     }

//     #[test]
//     fn parse_redis_input_status_is_reblog() -> Result<(), Err> {
//         let input = "*3\r\n$7\r\nmessage\r\n$12\r\ntimeline:308\r\n$2778\r\n{\"event\":\"update\",\"payload\":{\"id\":\"103885156768039822\",\"created_at\":\"2020-03-25T18:41:33.859Z\",\"in_reply_to_id\":null,\"in_reply_to_account_id\":null,\"sensitive\":false,\"spoiler_text\":\"\",\"visibility\":\"public\",\"language\":null,\"uri\":\"https://instance.codesections.com/users/ralph/statuses/103885156768039822/activity\",\"url\":\"https://instance.codesections.com/users/ralph/statuses/103885156768039822/activity\",\"replies_count\":0,\"reblogs_count\":0,\"favourites_count\":0,\"favourited\":false,\"reblogged\":true,\"muted\":false,\"bookmarked\":false,\"content\":\"<p>RT <span class=\\\"h-card\\\"><a href=\\\"https://instance.codesections.com/@bob\\\" class=\\\"u-url mention\\\">@<span>bob</span></a></span> 0010</p>\",\"reblog\":{\"id\":\"103881061540314589\",\"created_at\":\"2020-03-25T01:20:05.648Z\",\"in_reply_to_id\":null,\"in_reply_to_account_id\":null,\"sensitive\":false,\"spoiler_text\":\"\",\"visibility\":\"public\",\"language\":\"en\",\"uri\":\"https://instance.codesections.com/users/bob/statuses/103881061540314589\",\"url\":\"https://instance.codesections.com/@bob/103881061540314589\",\"replies_count\":0,\"reblogs_count\":1,\"favourites_count\":0,\"favourited\":false,\"reblogged\":true,\"muted\":false,\"bookmarked\":false,\"content\":\"<p>0010</p>\",\"reblog\":null,\"application\":{\"name\":\"Web\",\"website\":null},\"account\":{\"id\":\"55\",\"username\":\"bob\",\"acct\":\"bob\",\"display_name\":\"\",\"locked\":false,\"bot\":false,\"discoverable\":null,\"group\":false,\"created_at\":\"2020-03-11T03:03:53.068Z\",\"note\":\"<p></p>\",\"url\":\"https://instance.codesections.com/@bob\",\"avatar\":\"https://instance.codesections.com/avatars/original/missing.png\",\"avatar_static\":\"https://instance.codesections.com/avatars/original/missing.png\",\"header\":\"https://instance.codesections.com/headers/original/missing.png\",\"header_static\":\"https://instance.codesections.com/headers/original/missing.png\",\"followers_count\":1,\"following_count\":1,\"statuses_count\":58,\"last_status_at\":\"2020-03-25\",\"emojis\":[],\"fields\":[]},\"media_attachments\":[],\"mentions\":[],\"tags\":[],\"emojis\":[],\"card\":null,\"poll\":null},\"application\":null,\"account\":{\"id\":\"308\",\"username\":\"ralph\",\"acct\":\"ralph\",\"display_name\":\"\",\"locked\":false,\"bot\":false,\"discoverable\":null,\"group\":false,\"created_at\":\"2020-03-11T19:55:20.933Z\",\"note\":\"<p></p>\",\"url\":\"https://instance.codesections.com/@ralph\",\"avatar\":\"https://instance.codesections.com/avatars/original/missing.png\",\"avatar_static\":\"https://instance.codesections.com/avatars/original/missing.png\",\"header\":\"https://instance.codesections.com/headers/original/missing.png\",\"header_static\":\"https://instance.codesections.com/headers/original/missing.png\",\"followers_count\":1,\"following_count\":2,\"statuses_count\":110,\"last_status_at\":\"2020-03-25\",\"emojis\":[],\"fields\":[]},\"media_attachments\":[],\"mentions\":[],\"tags\":[],\"emojis\":[],\"card\":null,\"poll\":null},\"queued_at\":1585161694429}\r\n";

//         let (mut cache, _, _, _) = shared_setup();

//         let (msg, rest) = RedisMsg::from_raw(input, &mut cache, &None)?;
//         dbg!(&msg);
//         assert!(matches!(
//             msg,
//             RedisMsg::EventMsg(
//                 Timeline(User(308), Federated, All),
//                 Event::Update {
//                     payload:
//                         Status {
//                             reblogged: Some(t), ..
//                         },
//                     ..
//                 },
//             ) if t
//         ));

//         assert_eq!(rest, "".to_string());
//         Ok(())
//     }

//     #[test]
//     fn parse_redis_input_status_with_poll() -> Result<(), Err> {
//         let input = "*3\r\n$7\r\nmessage\r\n$12\r\ntimeline:308\r\n$1663\r\n{\"event\":\"update\",\"payload\":{\"id\":\"103885228849512739\",\"created_at\":\"2020-03-25T18:59:53.788Z\",\"in_reply_to_id\":null,\"in_reply_to_account_id\":null,\"sensitive\":false,\"spoiler_text\":\"\",\"visibility\":\"public\",\"language\":\"en\",\"uri\":\"https://instance.codesections.com/users/ralph/statuses/103885228849512739\",\"url\":\"https://instance.codesections.com/@ralph/103885228849512739\",\"replies_count\":0,\"reblogs_count\":0,\"favourites_count\":0,\"favourited\":false,\"reblogged\":false,\"muted\":false,\"bookmarked\":false,\"pinned\":false,\"content\":\"<p>test poll:</p>\",\"reblog\":null,\"application\":{\"name\":\"Web\",\"website\":null},\"account\":{\"id\":\"308\",\"username\":\"ralph\",\"acct\":\"ralph\",\"display_name\":\"\",\"locked\":false,\"bot\":false,\"discoverable\":null,\"group\":false,\"created_at\":\"2020-03-11T19:55:20.933Z\",\"note\":\"<p></p>\",\"url\":\"https://instance.codesections.com/@ralph\",\"avatar\":\"https://instance.codesections.com/avatars/original/missing.png\",\"avatar_static\":\"https://instance.codesections.com/avatars/original/missing.png\",\"header\":\"https://instance.codesections.com/headers/original/missing.png\",\"header_static\":\"https://instance.codesections.com/headers/original/missing.png\",\"followers_count\":1,\"following_count\":2,\"statuses_count\":112,\"last_status_at\":\"2020-03-25\",\"emojis\":[],\"fields\":[]},\"media_attachments\":[],\"mentions\":[],\"tags\":[],\"emojis\":[],\"card\":null,\"poll\":{\"id\":\"46\",\"expires_at\":\"2020-03-26T18:59:53.747Z\",\"expired\":false,\"multiple\":false,\"votes_count\":0,\"voters_count\":0,\"voted\":true,\"own_votes\":[],\"options\":[{\"title\":\"1\",\"votes_count\":0},{\"title\":\"2\",\"votes_count\":0},{\"title\":\"3\",\"votes_count\":0},{\"title\":\"4\",\"votes_count\":0}],\"emojis\":[]}},\"queued_at\":1585162794362}\r\n";

//         let (mut cache, _, _, _) = shared_setup();

//         let (msg, rest) = RedisMsg::from_raw(input, &mut cache, &None)?;
//         dbg!(&msg);
//         assert!(matches!(
//             msg,
//             RedisMsg::EventMsg(
//                 Timeline(User(308), Federated, All),
//                 Event::Update {
//                     payload: Status { poll: Some(_), .. },
//                     ..
//                 },
//             )
//         ));

//         assert_eq!(rest, "".to_string());
//         Ok(())
//     }

//     #[test]
//     fn parse_redis_input_status_with_preview_card() -> Result<(), Err> {
//         let input = "*3\r\n$7\r\nmessage\r\n$11\r\ntimeline:55\r\n$2256\r\n{\"event\":\"update\",\"payload\":{\"id\":\"103885300935387207\",\"created_at\":\"2020-03-25T19:18:13.753Z\",\"in_reply_to_id\":null,\"in_reply_to_account_id\":null,\"sensitive\":false,\"spoiler_text\":\"\",\"visibility\":\"public\",\"language\":\"en\",\"uri\":\"https://instance.codesections.com/users/ralph/statuses/103885300935387207\",\"url\":\"https://instance.codesections.com/@ralph/103885300935387207\",\"replies_count\":0,\"reblogs_count\":0,\"favourites_count\":0,\"favourited\":false,\"reblogged\":false,\"muted\":false,\"bookmarked\":false,\"content\":\"<p>Test with preview card:</p><p><a href=\\\"https://www.codesections.com/blog/mastodon-elevator-pitch/\\\" rel=\\\"nofollow noopener noreferrer\\\" target=\\\"_blank\\\"><span class=\\\"invisible\\\">https://www.</span><span class=\\\"ellipsis\\\">codesections.com/blog/mastodon</span><span class=\\\"invisible\\\">-elevator-pitch/</span></a></p>\",\"reblog\":null,\"application\":{\"name\":\"Web\",\"website\":null},\"account\":{\"id\":\"308\",\"username\":\"ralph\",\"acct\":\"ralph\",\"display_name\":\"\",\"locked\":false,\"bot\":false,\"discoverable\":null,\"group\":false,\"created_at\":\"2020-03-11T19:55:20.933Z\",\"note\":\"<p></p>\",\"url\":\"https://instance.codesections.com/@ralph\",\"avatar\":\"https://instance.codesections.com/avatars/original/missing.png\",\"avatar_static\":\"https://instance.codesections.com/avatars/original/missing.png\",\"header\":\"https://instance.codesections.com/headers/original/missing.png\",\"header_static\":\"https://instance.codesections.com/headers/original/missing.png\",\"followers_count\":2,\"following_count\":2,\"statuses_count\":120,\"last_status_at\":\"2020-03-25\",\"emojis\":[],\"fields\":[]},\"media_attachments\":[],\"mentions\":[],\"tags\":[],\"emojis\":[],\"card\":{\"url\":\"https://www.codesections.com/blog/mastodon-elevator-pitch/\",\"title\":\" Mastodon Is Better than Twitter: Elevator Pitch |  CodeSections\",\"description\":\"The personal website and blog of Daniel Long Sockwell, a lawyer-turned-programmer with an interest in web development, open source, and making things as simple as possible.\",\"type\":\"link\",\"author_name\":\"\",\"author_url\":\"\",\"provider_name\":\"\",\"provider_url\":\"\",\"html\":\"\",\"width\":400,\"height\":400,\"image\":\"https://instance.codesections.com/system/preview_cards/images/000/000/002/original/f6e89baa729668e7.png?1585163010\",\"embed_url\":\"\"},\"poll\":null},\"queued_at\":1585163894281}\r\n";

//         let (mut cache, _, _, _) = shared_setup();

//         let (msg, rest) = RedisMsg::from_raw(input, &mut cache, &None)?;
//         dbg!(&msg);
//         assert!(matches!(
//             msg,
//             RedisMsg::EventMsg(
//                 Timeline(User(55), Federated, All),
//                 Event::Update {
//                     payload: Status { card: Some(_), .. },
//                     ..
//                 },
//             )
//         ));

//         assert_eq!(rest, "".to_string());
//         Ok(())
//     }

//     #[test]
//     fn parse_redis_input_conversation() -> Result<(), Err> {
//         let input = "*3\r\n$7\r\nmessage\r\n$17\r\ntimeline:direct:9\r\n$2442\r\n{\"event\":\"conversation\",\"payload\":{\"id\":\"22\",\"unread\":false,\"accounts\":[{\"id\":\"55\",\"username\":\"bob\",\"acct\":\"bob\",\"display_name\":\"\",\"locked\":false,\"bot\":false,\"discoverable\":null,\"group\":false,\"created_at\":\"2020-03-11T03:03:53.068Z\",\"note\":\"<p></p>\",\"url\":\"https://instance.codesections.com/@bob\",\"avatar\":\"https://instance.codesections.com/avatars/original/missing.png\",\"avatar_static\":\"https://instance.codesections.com/avatars/original/missing.png\",\"header\":\"https://instance.codesections.com/headers/original/missing.png\",\"header_static\":\"https://instance.codesections.com/headers/original/missing.png\",\"followers_count\":1,\"following_count\":1,\"statuses_count\":58,\"last_status_at\":\"2020-03-25\",\"emojis\":[],\"fields\":[]}],\"last_status\":{\"id\":\"103884351200485419\",\"created_at\":\"2020-03-25T15:16:41.915Z\",\"in_reply_to_id\":null,\"in_reply_to_account_id\":null,\"sensitive\":false,\"spoiler_text\":\"\",\"visibility\":\"direct\",\"language\":\"en\",\"uri\":\"https://instance.codesections.com/users/codesections/statuses/103884351200485419\",\"url\":\"https://instance.codesections.com/@codesections/103884351200485419\",\"replies_count\":0,\"reblogs_count\":0,\"favourites_count\":0,\"favourited\":false,\"reblogged\":false,\"muted\":false,\"bookmarked\":false,\"content\":\"<p><span class=\\\"h-card\\\"><a href=\\\"https://instance.codesections.com/@bob\\\" class=\\\"u-url mention\\\">@<span>bob</span></a></span> Test Conversation</p>\",\"reblog\":null,\"application\":{\"name\":\"Web\",\"website\":null},\"account\":{\"id\":\"9\",\"username\":\"codesections\",\"acct\":\"codesections\",\"display_name\":\"TEST ACCOUT for codesections\",\"locked\":false,\"bot\":false,\"discoverable\":false,\"group\":false,\"created_at\":\"2020-03-11T01:17:13.412Z\",\"note\":\"<p>Used in the testing and development of flodgatt, the WIP streaming server for Mastodon</p>\",\"url\":\"https://instance.codesections.com/@codesections\",\"avatar\":\"https://instance.codesections.com/avatars/original/missing.png\",\"avatar_static\":\"https://instance.codesections.com/avatars/original/missing.png\",\"header\":\"https://instance.codesections.com/headers/original/missing.png\",\"header_static\":\"https://instance.codesections.com/headers/original/missing.png\",\"followers_count\":79,\"following_count\":97,\"statuses_count\":7,\"last_status_at\":\"2020-03-25\",\"emojis\":[],\"fields\":[]},\"media_attachments\":[],\"mentions\":[{\"id\":\"55\",\"username\":\"bob\",\"url\":\"https://instance.codesections.com/@bob\",\"acct\":\"bob\"}],\"tags\":[],\"emojis\":[],\"card\":null,\"poll\":null}},\"queued_at\":1585149402344}\r\n";

//         let (mut cache, _, _, _) = shared_setup();

//         let (msg, rest) = RedisMsg::from_raw(input, &mut cache, &None)?;
//         dbg!(&msg);
//         assert!(matches!(
//             msg,
//             RedisMsg::EventMsg(
//                 Timeline(Direct(id), Federated, All),
//                 Event::Conversation{ ..}
//             ) if id == 9
//         ));

//         assert_eq!(rest, "".to_string());
//         Ok(())
//     }

//     #[test]
//     fn parse_redis_input_from_live_data_1() -> Result<(), Err> {
//         let input = "*3\r\n$7\r\nmessage\r\n$15\r\ntimeline:public\r\n$2799\r\n{\"event\":\"update\",\"payload\":{\"id\":\"103880088450458596\",\"created_at\":\"2020-03-24T21:12:37.000Z\",\"in_reply_to_id\":null,\"in_reply_to_account_id\":null,\"sensitive\":false,\"spoiler_text\":\"\",\"visibility\":\"public\",\"language\":\"es\",\"uri\":\"https://mastodon.social/users/durru/statuses/103880088436492032\",\"url\":\"https://mastodon.social/@durru/103880088436492032\",\"replies_count\":0,\"reblogs_count\":0,\"favourites_count\":0,\"content\":\"<p>¡No puedes salir, loca!</p>\",\"reblog\":null,\"account\":{\"id\":\"2271\",\"username\":\"durru\",\"acct\":\"durru@mastodon.social\",\"display_name\":\"Cloaca Maxima\",\"locked\":false,\"bot\":false,\"discoverable\":true,\"group\":false,\"created_at\":\"2020-03-24T21:27:31.669Z\",\"note\":\"<p>Todo pasa, antes o después, por la Cloaca, diría Vitruvio.<br>También compongo palíndromos.</p>\",\"url\":\"https://mastodon.social/@durru\",\"avatar\":\"https://instance.codesections.com/system/accounts/avatars/000/002/271/original/d7675a6ff9d9baa7.jpeg?1585085250\",\"avatar_static\":\"https://instance.codesections.com/system/accounts/avatars/000/002/271/original/d7675a6ff9d9baa7.jpeg?1585085250\",\"header\":\"https://instance.codesections.com/system/accounts/headers/000/002/271/original/e3f0a1989b0d8efc.jpeg?1585085250\",\"header_static\":\"https://instance.codesections.com/system/accounts/headers/000/002/271/original/e3f0a1989b0d8efc.jpeg?1585085250\",\"followers_count\":222,\"following_count\":81,\"statuses_count\":5443,\"last_status_at\":\"2020-03-24\",\"emojis\":[],\"fields\":[{\"name\":\"Mis fotos\",\"value\":\"<a href=\\\"https://pixelfed.de/durru\\\" rel=\\\"nofollow noopener noreferrer\\\" target=\\\"_blank\\\"><span class=\\\"invisible\\\">https://</span><span class=\\\"\\\">pixelfed.de/durru</span><span class=\\\"invisible\\\"></span></a>\",\"verified_at\":null},{\"name\":\"diaspora*\",\"value\":\"<a href=\\\"https://joindiaspora.com/people/75fec0e05114013484870242ac110007\\\" rel=\\\"nofollow noopener noreferrer\\\" target=\\\"_blank\\\"><span class=\\\"invisible\\\">https://</span><span class=\\\"ellipsis\\\">joindiaspora.com/people/75fec0</span><span class=\\\"invisible\\\">e05114013484870242ac110007</span></a>\",\"verified_at\":null}]},\"media_attachments\":[{\"id\":\"2864\",\"type\":\"image\",\"url\":\"https://instance.codesections.com/system/media_attachments/files/000/002/864/original/3988312d30936494.jpeg?1585085251\",\"preview_url\":\"https://instance.codesections.com/system/media_attachments/files/000/002/864/small/3988312d30936494.jpeg?1585085251\",\"remote_url\":\"https://files.mastodon.social/media_attachments/files/026/669/690/original/d8171331f956cf38.jpg\",\"text_url\":null,\"meta\":{\"original\":{\"width\":1001,\"height\":662,\"size\":\"1001x662\",\"aspect\":1.512084592145015},\"small\":{\"width\":491,\"height\":325,\"size\":\"491x325\",\"aspect\":1.5107692307692309}},\"description\":null,\"blurhash\":\"UdLqhI4n4TIUIAt7t7ay~qIojtRj?bM{M{of\"}],\"mentions\":[],\"tags\":[],\"emojis\":[],\"card\":null,\"poll\":null}}\r\n";
//         let (mut cache, _, _, _) = shared_setup();
//         let (msg, rest) = RedisMsg::from_raw(input, &mut cache, &None)?;
//         assert!(matches!(
//             msg,
//             RedisMsg::EventMsg(Timeline(Public, Federated, All), Event::Update { .. })
//         ));
//         assert_eq!(rest, String::new());
//         Ok(())
//     }

//     #[test]
//     fn parse_redis_input_from_live_data_2() -> Result<(), Err> {
//         let input = "*3\r\n$7\r\nmessage\r\n$15\r\ntimeline:public\r\n$3888\r\n{\"event\":\"update\",\"payload\":{\"id\":\"103880373579328660\",\"created_at\":\"2020-03-24T22:25:05.000Z\",\"in_reply_to_id\":null,\"in_reply_to_account_id\":null,\"sensitive\":false,\"spoiler_text\":\"\",\"visibility\":\"public\",\"language\":\"en\",\"uri\":\"https://newsbots.eu/users/granma/statuses/103880373417385978\",\"url\":\"https://newsbots.eu/@granma/103880373417385978\",\"replies_count\":0,\"reblogs_count\":0,\"favourites_count\":0,\"content\":\"<p>A total of 11 measures have been established for the pre-epidemic stage of the battle against <a href=\\\"https://newsbots.eu/tags/Covid\\\" class=\\\"mention hashtag\\\" rel=\\\"nofollow noopener noreferrer\\\" target=\\\"_blank\\\">#<span>Covid</span></a>-19 in <a href=\\\"https://newsbots.eu/tags/Cuba\\\" class=\\\"mention hashtag\\\" rel=\\\"nofollow noopener noreferrer\\\" target=\\\"_blank\\\">#<span>Cuba</span></a> <br><a href=\\\"https://newsbots.eu/tags/CubaPorLaSalud\\\" class=\\\"mention hashtag\\\" rel=\\\"nofollow noopener noreferrer\\\" target=\\\"_blank\\\">#<span>CubaPorLaSalud</span></a> <br> <a href=\\\"http://en.granma.cu/cuba/2020-03-23/public-health-measures-in-covid-19-pre-epidemic-stage\\\" rel=\\\"nofollow noopener noreferrer\\\" target=\\\"_blank\\\"><span class=\\\"invisible\\\">http://</span><span class=\\\"ellipsis\\\">en.granma.cu/cuba/2020-03-23/p</span><span class=\\\"invisible\\\">ublic-health-measures-in-covid-19-pre-epidemic-stage</span></a>&nbsp;</p>\",\"reblog\":null,\"account\":{\"id\":\"717\",\"username\":\"granma\",\"acct\":\"granma@newsbots.eu\",\"display_name\":\"Granma (Unofficial)\",\"locked\":false,\"bot\":true,\"discoverable\":false,\"group\":false,\"created_at\":\"2020-03-13T11:08:08.420Z\",\"note\":\"<p></p>\",\"url\":\"https://newsbots.eu/@granma\",\"avatar\":\"https://instance.codesections.com/system/accounts/avatars/000/000/717/original/4a1f9ed090fc36e9.jpeg?1584097687\",\"avatar_static\":\"https://instance.codesections.com/system/accounts/avatars/000/000/717/original/4a1f9ed090fc36e9.jpeg?1584097687\",\"header\":\"https://instance.codesections.com/headers/original/missing.png\",\"header_static\":\"https://instance.codesections.com/headers/original/missing.png\",\"followers_count\":57,\"following_count\":1,\"statuses_count\":742,\"last_status_at\":\"2020-03-24\",\"emojis\":[],\"fields\":[{\"name\":\"Source\",\"value\":\"<a href=\\\"https://twitter.com/Granma_English\\\" rel=\\\"nofollow noopener noreferrer\\\" target=\\\"_blank\\\"><span class=\\\"invisible\\\">https://</span><span class=\\\"\\\">twitter.com/Granma_English</span><span class=\\\"invisible\\\"></span></a>\",\"verified_at\":null},{\"name\":\"Operator\",\"value\":\"<span class=\\\"h-card\\\"><a href=\\\"https://radical.town/@felix\\\" class=\\\"u-url mention\\\" rel=\\\"nofollow noopener noreferrer\\\" target=\\\"_blank\\\">@<span>felix</span></a></span>\",\"verified_at\":null},{\"name\":\"Code\",\"value\":\"<a href=\\\"https://yerbamate.dev/nutomic/tootbot\\\" rel=\\\"nofollow noopener noreferrer\\\" target=\\\"_blank\\\"><span class=\\\"invisible\\\">https://</span><span class=\\\"\\\">yerbamate.dev/nutomic/tootbot</span><span class=\\\"invisible\\\"></span></a>\",\"verified_at\":null}]},\"media_attachments\":[{\"id\":\"2881\",\"type\":\"image\",\"url\":\"https://instance.codesections.com/system/media_attachments/files/000/002/881/original/a1e97908e84efbcd.jpeg?1585088707\",\"preview_url\":\"https://instance.codesections.com/system/media_attachments/files/000/002/881/small/a1e97908e84efbcd.jpeg?1585088707\",\"remote_url\":\"https://newsbots.eu/system/media_attachments/files/000/176/298/original/f30a877d5035f4a6.jpeg\",\"text_url\":null,\"meta\":{\"original\":{\"width\":700,\"height\":795,\"size\":\"700x795\",\"aspect\":0.8805031446540881},\"small\":{\"width\":375,\"height\":426,\"size\":\"375x426\",\"aspect\":0.8802816901408451}},\"description\":null,\"blurhash\":\"UHCY?%sD%1t6}snOxuxu#7rrx]xu$*i_NFNF\"}],\"mentions\":[],\"tags\":[{\"name\":\"covid\",\"url\":\"https://instance.codesections.com/tags/covid\"},{\"name\":\"cuba\",\"url\":\"https://instance.codesections.com/tags/cuba\"},{\"name\":\"CubaPorLaSalud\",\"url\":\"https://instance.codesections.com/tags/CubaPorLaSalud\"}],\"emojis\":[],\"card\":null,\"poll\":null}}\r\n";
//         let (mut cache, _, _, _) = shared_setup();
//         let (msg, rest) = RedisMsg::from_raw(input, &mut cache, &None)?;
//         assert!(matches!(
//             msg,
//             RedisMsg::EventMsg(Timeline(Public, Federated, All), Event::Update { .. })
//         ));
//         assert_eq!(rest, String::new());
//         Ok(())
//     }

//     #[test]
//     fn parse_redis_input_from_live_data_3() -> Result<(), Err> {
//         let input = "*3\r\n$7\r\nmessage\r\n$15\r\ntimeline:public\r\n$4803\r\n{\"event\":\"update\",\"payload\":{\"id\":\"103880453908763088\",\"created_at\":\"2020-03-24T22:45:33.000Z\",\"in_reply_to_id\":null,\"in_reply_to_account_id\":null,\"sensitive\":false,\"spoiler_text\":\"\",\"visibility\":\"public\",\"language\":\"en\",\"uri\":\"https://mstdn.social/users/stux/statuses/103880453855603541\",\"url\":\"https://mstdn.social/@stux/103880453855603541\",\"replies_count\":0,\"reblogs_count\":0,\"favourites_count\":0,\"content\":\"<p>When they say lockdown. LOCKDOWN.</p>\",\"reblog\":null,\"account\":{\"id\":\"806\",\"username\":\"stux\",\"acct\":\"stux@mstdn.social\",\"display_name\":\"sтυx⚡\",\"locked\":false,\"bot\":false,\"discoverable\":true,\"group\":false,\"created_at\":\"2020-03-13T23:02:29.970Z\",\"note\":\"<p>Hi, Stux here! I am running the mstdn.social :mastodon: instance!</p><p>For questions and help or just for fun you can always send me a toot♥\u{fe0f}</p><p>Oh and no, I am not really a cat! Or am I?</p>\",\"url\":\"https://mstdn.social/@stux\",\"avatar\":\"https://instance.codesections.com/system/accounts/avatars/000/000/806/original/dae8d9d01d57d7f8.gif?1584140547\",\"avatar_static\":\"https://instance.codesections.com/system/accounts/avatars/000/000/806/static/dae8d9d01d57d7f8.png?1584140547\",\"header\":\"https://instance.codesections.com/system/accounts/headers/000/000/806/original/88c874d69f7d6989.gif?1584140548\",\"header_static\":\"https://instance.codesections.com/system/accounts/headers/000/000/806/static/88c874d69f7d6989.png?1584140548\",\"followers_count\":13954,\"following_count\":7600,\"statuses_count\":10207,\"last_status_at\":\"2020-03-24\",\"emojis\":[{\"shortcode\":\"mastodon\",\"url\":\"https://instance.codesections.com/system/custom_emojis/images/000/000/418/original/25ccc64333645735.png?1584140550\",\"static_url\":\"https://instance.codesections.com/system/custom_emojis/images/000/000/418/static/25ccc64333645735.png?1584140550\",\"visible_in_picker\":true},{\"shortcode\":\"patreon\",\"url\":\"https://instance.codesections.com/system/custom_emojis/images/000/000/419/original/3cc463d3dfc1e489.png?1584140550\",\"static_url\":\"https://instance.codesections.com/system/custom_emojis/images/000/000/419/static/3cc463d3dfc1e489.png?1584140550\",\"visible_in_picker\":true},{\"shortcode\":\"liberapay\",\"url\":\"https://instance.codesections.com/system/custom_emojis/images/000/000/420/original/893854353dfa9706.png?1584140551\",\"static_url\":\"https://instance.codesections.com/system/custom_emojis/images/000/000/420/static/893854353dfa9706.png?1584140551\",\"visible_in_picker\":true},{\"shortcode\":\"team_valor\",\"url\":\"https://instance.codesections.com/system/custom_emojis/images/000/000/958/original/96aae26b45292a12.png?1584910917\",\"static_url\":\"https://instance.codesections.com/system/custom_emojis/images/000/000/958/static/96aae26b45292a12.png?1584910917\",\"visible_in_picker\":true}],\"fields\":[{\"name\":\"Patreon :patreon:\",\"value\":\"<a href=\\\"https://www.patreon.com/mstdn\\\" rel=\\\"nofollow noopener noreferrer\\\" target=\\\"_blank\\\"><span class=\\\"invisible\\\">https://www.</span><span class=\\\"\\\">patreon.com/mstdn</span><span class=\\\"invisible\\\"></span></a>\",\"verified_at\":null},{\"name\":\"LiberaPay :liberapay:\",\"value\":\"<a href=\\\"https://liberapay.com/mstdn\\\" rel=\\\"nofollow noopener noreferrer\\\" target=\\\"_blank\\\"><span class=\\\"invisible\\\">https://</span><span class=\\\"\\\">liberapay.com/mstdn</span><span class=\\\"invisible\\\"></span></a>\",\"verified_at\":null},{\"name\":\"Team :team_valor:\",\"value\":\"<a href=\\\"https://mstdn.social/team\\\" rel=\\\"nofollow noopener noreferrer\\\" target=\\\"_blank\\\"><span class=\\\"invisible\\\">https://</span><span class=\\\"\\\">mstdn.social/team</span><span class=\\\"invisible\\\"></span></a>\",\"verified_at\":null},{\"name\":\"Support :mastodon:\",\"value\":\"<a href=\\\"https://mstdn.social/funding\\\" rel=\\\"nofollow noopener noreferrer\\\" target=\\\"_blank\\\"><span class=\\\"invisible\\\">https://</span><span class=\\\"\\\">mstdn.social/funding</span><span class=\\\"invisible\\\"></span></a>\",\"verified_at\":null}]},\"media_attachments\":[{\"id\":\"2886\",\"type\":\"video\",\"url\":\"https://instance.codesections.com/system/media_attachments/files/000/002/886/original/22b3f98a5e8f86d8.mp4?1585090023\",\"preview_url\":\"https://instance.codesections.com/system/media_attachments/files/000/002/886/small/22b3f98a5e8f86d8.png?1585090023\",\"remote_url\":\"https://cdn.mstdn.social/mstdn-social/media_attachments/files/003/338/384/original/c146f62ba86fe63e.mp4\",\"text_url\":null,\"meta\":{\"length\":\"0:00:27.03\",\"duration\":27.03,\"fps\":30,\"size\":\"272x480\",\"width\":272,\"height\":480,\"aspect\":0.5666666666666667,\"audio_encode\":\"aac (LC) (mp4a / 0x6134706D)\",\"audio_bitrate\":\"44100 Hz\",\"audio_channels\":\"stereo\",\"original\":{\"width\":272,\"height\":480,\"frame_rate\":\"30/1\",\"duration\":27.029,\"bitrate\":481885},\"small\":{\"width\":227,\"height\":400,\"size\":\"227x400\",\"aspect\":0.5675}},\"description\":null,\"blurhash\":\"UBF~N@OF-:xv4mM|s+ob9FE2t6tQ9Fs:t8oN\"}],\"mentions\":[],\"tags\":[],\"emojis\":[],\"card\":null,\"poll\":null}}\r\n";
//         let (mut cache, _, _, _) = shared_setup();
//         let (msg, rest) = RedisMsg::from_raw(input, &mut cache, &None)?;
//         assert!(matches!(
//             msg,
//             RedisMsg::EventMsg(Timeline(Public, Federated, All), Event::Update { .. })
//         ));
//         assert_eq!(rest, String::new());
//         Ok(())
//     }
