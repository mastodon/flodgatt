# RageQuit
A blazingly fast drop-in replacement for the Mastodon streaming api server

## Notes on data flow

The current structure of the app is as follows:

Client Request --> Warp 
   Warp filters for valid requests and parses request data.  Based on that data, it repeatedly polls the StreamManager

Warp --> StreamManager
   The StreamManager consults a hash table to see if there is a currently open PubSub channel.  If there is, it uses that channel; if not, it (synchronously) sends a subscribe command to Redis.  The StreamManager polls the Receiver, providing info about which StreamManager it is that is doing the polling.  The stream manager is also responsible for monitoring the hash table to see if it should unsubscribe from any channels and, if necessary, sending the unsubscribe command. 
   
StreamManger --> Receiver 
   The Receiver receives data from Redis and stores it in a series of queues (one for each StreamManager).  When (asynchronously) polled by the StreamManager, it sends back the messages relevant to that StreamManager and removes them from the queue.
