//! Native runtime logging configuration.
//!
//! This is a cloneable, source-agnostic mirror of the fields the application's logging stack needs.
//! The binary maps it into the logging stack's own configuration type at the edge. Keeping a native
//! type here avoids depending on the application crate and keeps logging reloads expressible purely
//! in terms of [`SalukiConfiguration`](crate::saluki::SalukiConfiguration).

use bytesize::ByteSize;

/// Runtime logging configuration after translation.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RuntimeLoggingConfig {
    /// Verbosity directive (for example `info`, `debug`, or a per-target filter string). `None`
    /// means "use the application default".
    pub log_level: Option<String>,

    /// Whether to emit log records as JSON.
    pub log_format_json: bool,

    /// Whether to use RFC 3339 timestamps.
    pub log_format_rfc3339: bool,

    /// Whether to write to standard output.
    pub log_to_console: bool,

    /// Whether to write to syslog.
    pub log_to_syslog: bool,

    /// Whether to use the RFC-style syslog header.
    pub syslog_rfc: bool,

    /// Syslog destination URI (empty means "platform default" when syslog is enabled).
    pub syslog_uri: String,

    /// Log file path, or empty to disable file logging.
    pub log_file: String,

    /// Maximum size of a log file before it rolls over.
    pub log_file_max_size: ByteSize,

    /// Maximum number of rolled log files retained.
    pub log_file_max_rolls: usize,
}

impl Default for RuntimeLoggingConfig {
    fn default() -> Self {
        Self {
            log_level: None,
            log_format_json: false,
            log_format_rfc3339: false,
            log_to_console: true,
            log_to_syslog: false,
            syslog_rfc: false,
            syslog_uri: String::new(),
            log_file: String::new(),
            log_file_max_size: ByteSize::mib(10),
            log_file_max_rolls: 1,
        }
    }
}
