//! `sqlx::migrate!` embeds the migration files at compile time, and cargo does
//! not watch a directory for new files on its own: adding a migration left the
//! old set compiled in until something else touched the crate. The indexer's
//! set is watched too, because this crate's tests migrate it.
fn main() {
    println!("cargo:rerun-if-changed=migrations");
    println!("cargo:rerun-if-changed=../x3-indexer/migrations");
}
