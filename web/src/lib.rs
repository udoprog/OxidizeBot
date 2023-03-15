//! <div align="center">
//!     <a href="https://setbac.tv">
//!         <img src="https://raw.githubusercontent.com/udoprog/OxidizeBot/main/bot/res/icon48.png" title="Oxidize Bot">
//!     </a>
//! </div>
//!
//! <p align="center">
//!     The web component of OxidizeBot, a high performance Twitch Bot powered by Rust
//! </p>
//!
//! <div align="center">
//!     <a href="https://github.com/udoprog/OxidizeBot"><img alt="github" src="https://img.shields.io/badge/github-udoprog/OxidizeBot-8da0cb?style=for-the-badge&logo=github" height="20"></a>
//!     <a href="https://github.com/udoprog/OxidizeBot/actions?query=branch%3Amain"><img alt="build status" src="https://img.shields.io/github/actions/workflow/status/udoprog/OxidizeBot/ci.yml?branch=main&style=for-the-badge" height="20"></a>
//!     <a href="https://discord.gg/v5AeNkT"><img alt="chat on discord" src="https://img.shields.io/discord/558644981137670144.svg?logo=discord&style=flat-square" height="20"></a>
//! </div>
//!
//! <div align="center">
//!     <a href="https://setbac.tv/" rel="nofollow">Site 🌐</a>
//!     &ndash;
//!     <a href="https://setbac.tv/help" rel="nofollow">Command Help ❓</a>
//! </div>
//!
//! <br>
//!
#![recursion_limit = "256"]

mod aead;
pub mod api;
pub mod db;
mod oauth2;
mod session;
pub mod web;

pub(crate) use tokio_stream as stream;
