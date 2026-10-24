//! Promotional videos, made from scenarios (`contrib/promo/scenarios/`):
//! `contrib/promo/make <scenario>` runs [`runner`] for the app's frames and
//! what happened in each, then `contrib/promo/render.cjs` for the video.
//! The design: docs/superpowers/specs/2026-10-24-promo-video-scenarios-design.md.

mod scenario;

// Tasks 3 and 4 create these:
// #[cfg(feature = "shots")]
// mod live;
// #[cfg(feature = "shots")]
// mod runner;
