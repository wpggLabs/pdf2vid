# Contributing

1. Open an issue before large architectural changes.
2. Keep providers behind the provider registry and native command boundary.
3. Never log API keys, PDF contents, or generated narration.
4. Run the frontend build, frontend tests, and Rust tests before submitting.
5. Keep pull requests focused and document user-visible behavior.

Paid providers must never be selected automatically over a free local provider.
