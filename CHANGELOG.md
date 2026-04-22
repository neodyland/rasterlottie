# Changelog

All notable changes to this project will be documented in this file.

The format is based on Keep a Changelog, and the project follows Semantic Versioning.

## [0.1.1]

### Added

- `#[cfg_attr(docsrs, feature(doc_cfg))]` attribute for documentation generation.
- 3 Badges for crates.io, docs.rs, and GitHub Actions status.
- GitHub Actions workflows for CI verification and crates.io publishing.
- This changelog.

### Changed

- Restricted publishing to crates.io in package metadata.
- Expanded packaging and release metadata for crates.io publication.

## [0.1.0]

### Added

- Initial public release of `rasterlottie`.
- Deterministic support analysis for the current target corpus.
- Headless RGBA frame rendering backed by `tiny-skia`.
- GIF export for supported animations.
- Optional image asset support for embedded and resolver-backed external images.
- Optional glyph-backed text rendering support.
- Corpus, fixture, and image-diff regression coverage for the supported subset.
