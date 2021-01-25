extern crate git_rev;

fn main() {
    println!("Revision string: {:?}", git_rev::try_revision_string!());
    if let Some(rev) = git_rev::try_revision_u64!() {
        println!("Revision u64: {:016x}", rev);
    }
}
