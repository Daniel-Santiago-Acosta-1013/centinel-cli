use sentinel::vpn::dns_override::parse_hwports;
use std::collections::HashMap;

#[test]
fn parse_hwports_sample() {
    let sample = r#"
Hardware Port: Wi-Fi
Device: en0
Ethernet Address: aa:bb:cc:dd:ee:ff

Hardware Port: Thunderbolt Ethernet
Device: en5
Ethernet Address: ff:ee:dd:cc:bb:aa
    "#;
    let map = parse_hwports(sample);
    let mut expected = HashMap::new();
    expected.insert("en0".to_string(), "Wi-Fi".to_string());
    expected.insert("en5".to_string(), "Thunderbolt Ethernet".to_string());
    assert_eq!(map, expected);
}
