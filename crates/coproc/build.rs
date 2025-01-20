//! We get some git metadata at build time so we can log the info
//! at server start up

fn main() {
    let git2 = vergen_git2::Git2Builder::all_git().unwrap();
    vergen_git2::Emitter::default().add_instructions(&git2).unwrap().emit().unwrap();
}
