//! `sqlx::migrate!` embeds the migration files at compile time; without this,
//! cargo does not rebuild when a migration is added to the directory.
fn main() {
    println!("cargo:rerun-if-changed=migrations");
}
