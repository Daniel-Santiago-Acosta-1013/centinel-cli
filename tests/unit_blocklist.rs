use sentinel::vpn_testing::load_blocklist;
use std::fs;
use tempfile::tempdir;

#[test]
fn blocklist_merges_and_deduplicates() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("custom.txt");
    fs::write(
        &path,
        b"example.com\nads.google.com\n# comentario\nExample.COM\n",
    )
    .unwrap();

    let list = load_blocklist(Some(path.as_path())).unwrap();
    // Debe deduplicar y normalizar.
    assert!(list.contains(&"example.com".to_string()));
    assert!(list.contains(&"ads.google.com".to_string()));
    // La base incluye doubleclick.net (sanity)
    assert!(list.contains(&"doubleclick.net".to_string()));
}
