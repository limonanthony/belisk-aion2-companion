//! Embeds the Windows manifest in the binary. Capture needs Administrator, so
//! the shipped binary asks for it; a build with the `probes` feature asks for
//! nothing, so the development checks run from an ordinary terminal.

fn main() {
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("windows") {
        return;
    }
    let resource = if std::env::var_os("CARGO_FEATURE_PROBES").is_some() {
        "windows/unelevated.rc"
    } else {
        "windows/admin.rc"
    };
    println!("cargo:rerun-if-changed=windows");
    embed_resource::compile(resource, embed_resource::NONE)
        .manifest_required()
        .unwrap();
}
