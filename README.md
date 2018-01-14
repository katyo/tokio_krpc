KRPC protocol in Rust
=====================

Kademlia Remote Procedure Call (KRPC) protocol definition from
[BEP-0005](http://www.bittorrent.org/beps/bep_0005.html):

> The KRPC protocol is a simple RPC mechanism consisting of bencoded dictionaries sent over UDP.
> A single query packet is sent out and a single packet is sent in response. There is no retry.
> There are three message types: query, response, and error.

Implementation notes
--------------------

The API centered around two service interfaces:

1. The service which can be called to make outgoing queries to another nodes.
2. The service which may handle the incoming queries from another nodes.

See DHT ping example here: [tests/dht-bittorrent.rs](tests/dht-bittorrent.rs)

Currently this library developed in single threaded manner to avoid synchronization overhead.
