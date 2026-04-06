# DogStatsD Feature Parity

This document tracks feature parity between the Datadog Agent's DogStatsD implementation and Agent
Data Plane (ADP). Use it to determine whether ADP supports the DogStatsD features your workload
depends on.

## Status Legend

| Status | Meaning |
|--------|---------|
| **Implemented** | Feature is present in ADP and behaves the same as the Datadog Agent. |
| **Missing** | Feature exists in the Datadog Agent but is not yet implemented in ADP. |
| **Divergent** | Feature exists in both but behavior differs. See the notes column for details. |
| **ADP Only** | Feature is unique to ADP and does not exist in the Datadog Agent. |

## Metric Reception

| Config Key | Description | Status | Notes |
|------------|-------------|--------|-------|
| `dogstatsd_port` | UDP listen port | _TBD_ | |
| `dogstatsd_socket` | UDS datagram socket path | _TBD_ | |
| `dogstatsd_stream_socket` | UDS stream socket path | _TBD_ | |
| `dogstatsd_non_local_traffic` | Accept non-localhost UDP traffic | _TBD_ | |
| `dogstatsd_buffer_size` | Receive buffer size in bytes | _TBD_ | |

## Parsing and Decoding

| Config Key | Description | Status | Notes |
|------------|-------------|--------|-------|
| `dogstatsd_no_aggregation_pipeline` | Timestamp-based no-aggregation support | _TBD_ | |
| _more keys TBD_ | | | |

## Aggregation and Enrichment

| Config Key | Description | Status | Notes |
|------------|-------------|--------|-------|
| `dogstatsd_tag_cardinality` | Tag cardinality level for origin tags | _TBD_ | |
| _more keys TBD_ | | | |

## Forwarding and Serialization

| Config Key | Description | Status | Notes |
|------------|-------------|--------|-------|
| `forwarder_num_workers` | Concurrent forwarder workers | _TBD_ | |
| _more keys TBD_ | | | |

## General Infrastructure

Config keys that are not DogStatsD-specific but affect its behavior (API keys, proxy, TLS, etc.).

| Config Key | Description | Status | Notes |
|------------|-------------|--------|-------|
| `api_key` | Datadog API key used for metric submission | _TBD_ | |
| _more keys TBD_ | | | |

## ADP-Only Features

Features unique to Agent Data Plane that do not have a Datadog Agent equivalent.

| Config Key | Description | Notes |
|------------|-------------|-------|
| _TBD_ | | |
