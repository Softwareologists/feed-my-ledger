# Extending Cloud Service Support

To add a new spreadsheet backend implement the `CloudSpreadsheetService` trait.
At minimum you must provide methods for creating a sheet, appending rows,
reading rows and sharing the sheet with other users.

Once implemented you can optionally wrap the service with utilities such as
`BatchingCacheService` for caching or `RetryingService` for resiliency.

Adapters are placed under `src/cloud_adapters/` and re-exported in
`cloud_adapters::mod` so that they can be used by the rest of the crate.
