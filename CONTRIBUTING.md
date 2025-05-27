# Contributing to rusty-ledger

Thank you for considering contributing to **rusty-ledger**! Your contributions help improve the project and are greatly appreciated.

## Table of Contents

* [Code of Conduct](#code-of-conduct)
* [How Can I Contribute?](#how-can-i-contribute)

  * [Reporting Bugs](#reporting-bugs)
  * [Suggesting Enhancements](#suggesting-enhancements)
  * [Submitting Pull Requests](#submitting-pull-requests)
* [Development Setup](#development-setup)
* [Style Guide](#style-guide)
* [Commit Messages](#commit-messages)
* [License](#license)

## Code of Conduct

This project adheres to the [Contributor Covenant](https://www.contributor-covenant.org/) Code of Conduct. By participating, you are expected to uphold this code. Please report unacceptable behavior to [khanparijat@gmail.com](mailto:khanparijat@gmail.com).

## How Can I Contribute?

### Reporting Bugs

If you encounter a bug, please open an issue and include:

* A clear and descriptive title.
* Steps to reproduce the issue.
* Expected and actual results.
* Any relevant logs or screenshots.

### Suggesting Enhancements

We welcome suggestions for new features or improvements. When proposing an enhancement, please:

* Explain the motivation behind the suggestion.
* Describe the desired behavior.
* Provide examples or use cases.

### Submitting Pull Requests

To contribute code:

1. Fork the repository.
2. Create a new branch (`git checkout -b feature/YourFeature`).
3. Make your changes.
4. Ensure the code passes all tests (`cargo test`).
5. Commit your changes with a clear message.
6. Push to your fork (`git push origin feature/YourFeature`).
7. Open a pull request and describe your changes.

Please ensure your pull request:

* Is focused on a single feature or fix.
* Includes tests for new functionality.
* Adheres to the project's coding standards.

## Development Setup

To set up the development environment:

1. Install [Rust](https://www.rust-lang.org/tools/install).

2. Clone the repository:

   ```bash
   git clone https://github.com/yourusername/immutable-sheet-db.git
   cd immutable-sheet-db
   ```

3. Build the project:

   ```bash
   cargo build
   ```

4. Run tests:

   ```bash
   cargo test
   ```

## Style Guide

We follow Rust's standard formatting conventions. Please run `cargo fmt` before committing your changes.

## Commit Messages

Write clear and descriptive commit messages. Use the imperative mood (e.g., "Add feature" not "Added feature"). Include references to relevant issues when applicable.

## License

By contributing, you agree that your contributions will be licensed under the [MIT License](LICENSE).

---

Feel free to reach out with any questions or suggestions. Happy coding!
