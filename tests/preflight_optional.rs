use nix::unistd::Uid;
use sentinel::vpn_testing::preflight_check;

#[test]
fn preflight_runs_only_when_allowed() {
    if std::env::var("ALLOW_NET_TEST").ok().as_deref() != Some("1") {
        eprintln!("SKIP preflight (set ALLOW_NET_TEST=1 to run)");
        return;
    }
    if !Uid::effective().is_root() {
        eprintln!("SKIP preflight (needs root)");
        return;
    }
    preflight_check().expect("preflight debe pasar con internet disponible");
}
