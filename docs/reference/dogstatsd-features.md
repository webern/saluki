# DogStatsD Feature Parity

This document tracks feature parity between the Datadog Agent's DogStatsD implementation and Agent
Data Plane (ADP). Use it to determine whether ADP supports the DogStatsD features your workload
depends on.

Last updated: 2026-04-06

The `/dogstatsd-audit` skill can automatically inspect and update this, but it is also OK to edit this by hand.

## Status Legend

| Status | Meaning |
|--------|---------|
| **Implemented** | Feature is present in ADP and behaves the same as the Datadog Agent. |
| **Missing** | Feature exists in the Datadog Agent but is not yet implemented in ADP. |
| **Divergent** | Feature exists in both but behavior differs. See the notes column for details. |
| **ADP Only** | Feature is unique to ADP and does not exist in the Datadog Agent. |

## Features

| Config Key | Description | Status | Notes |
|------------|-------------|--------|-------|
| `dogstatsd_port` | UDP listen port | _TBD_ | |
| `dogstatsd_socket` | UDS datagram socket path | _TBD_ | |
| `dogstatsd_stream_socket` | UDS stream socket path | _TBD_ | |
| `dogstatsd_non_local_traffic` | Accept non-localhost UDP traffic | _TBD_ | |
| `dogstatsd_buffer_size` | Receive buffer size in bytes | _TBD_ | |
| `dogstatsd_no_aggregation_pipeline` | Timestamp-based no-aggregation support | _TBD_ | |
| `dogstatsd_tag_cardinality` | Tag cardinality level for origin tags | _TBD_ | |
| `forwarder_num_workers` | Concurrent forwarder workers | _TBD_ | |
| `api_key` | Datadog API key used for metric submission | _TBD_ | |

## Discussion

TODO: we will discuss features that are missing, divergent or otherwise notable

### dogstatsd_port

We have a diverfence as follows, in the Agent we see

```go
// blah some go code
```

and in ADP we see

```rust
// blah some Rust code
```

Blah blah discussion

## Action Items

Here we will create a check-box list of features to implement perhaps?
