# Configuring DogStatsD on Agent Data Plane

The DogStatsD implementation on ADP is has been redesigned in Rust for better resource guarantees and efficiency. Because the architecture is different from the original implementation, certain configuration values may be different, work slightly differently, or be planned but not yet implemented. The purpose of this page is to document these nuances.

Configuration values that are fully supported, and behave identically to the original implementation of DogStatsD are omitted from this page. If you find an error on this page, please [open an issue]!

<!-- @formatter:off -->
[open an issue]: https://github.com/DataDog/saluki/issues
<!-- @formatter:on -->
