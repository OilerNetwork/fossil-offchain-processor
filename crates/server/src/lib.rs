#![deny(unused_crate_dependencies)]
use sqlx as _;
use tower as _;
use tower_http as _;
use tracing_subscriber as _;
use uuid as _;

pub mod handlers;
pub mod middlewares;
pub mod pricing_data;
