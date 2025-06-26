# 1. Overview

This project provides a unified interface for applications to interact with cloud-based spreadsheet services (e.g., Google Sheets) as immutable, append-only databases. It ensures that once data is committed, it cannot be edited or deleted. Adjustments are made by appending new records, akin to double-entry bookkeeping.

# 2. Core Features

- Immutable Data Entries: Once data is committed, it becomes read-only.
- Append-Only Adjustments: Modifications are handled by appending new records that reference the original entries.
- Cloud Service Integration: Supports integration with services like Google Sheets.
- User Authentication: Users authenticate via OAuth2 to link their cloud accounts.
- Data Sharing: Users can share their data with others, controlling access permissions.

# 3. Technical Stack

- Programming Language: Rust, for its performance and safety.
- API Integration: Call the official Google Sheets REST API directly over HTTP, using `hyper` for the client implementation.
- Authentication: Implement OAuth2 for secure user authentication and authorization.
- Data Storage Format: Each record includes metadata such as timestamps and unique identifiers to maintain immutability.

# 4. Data Structure

Each entry in the spreadsheet represents a record with the following fields:

- Timestamp: Date and time of entry creation.
- Unique ID: A unique identifier for the record.
- Data Fields: User-defined fields containing the actual data.
- Reference ID: If the record is an adjustment, this field references the original record's ID.

# 5. User Interface

- Data Entry Form: Allows users to input new data.
- Commit Function: Finalizes the data entry, making it immutable.
- Adjustment Interface: Enables users to create adjustment records referencing existing entries.
- Sharing Controls: Users can manage access permissions for their data.

# 6. Performance Considerations

- Batch Operations: Implement batch processing for reading and writing to minimize API calls.
- Caching: Use local caching to reduce redundant data fetching.
- Concurrency: Handle concurrent data access and modifications gracefully.

# 7. Security Measures

- OAuth2 Authentication: Securely authenticate users and manage access tokens.
- Data Validation: Ensure all data conforms to expected formats before committing.
- Access Control: Respect and enforce the sharing permissions set by users.

# 8. Testing Strategy

- Unit Tests: Cover core functionalities, including data immutability, adjustment logic, and API interactions.
- Integration Tests: Test the complete workflow from data entry to storage and sharing.
- Mocking External Services: Use mocking libraries to simulate interactions with cloud services during testing.
- Continuous Integration: Set up CI pipelines to run tests on each commit and pull request.

# 9. Documentation

- README.md: Provide an overview of the project, setup instructions, and usage examples.
- API Documentation: Detail the public interfaces and data structures.
- Contributing Guidelines: Outline the process for contributing to the project, including coding standards and submission procedures.
