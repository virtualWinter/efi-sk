use std::{collections::BTreeMap, env, fs, path::PathBuf};

fn main() {
    let source = "assets/distro/index.tsv";
    println!("cargo:rerun-if-changed={source}");
    let data = fs::read_to_string(source).expect("read distro icon index");
    let mut aliases = BTreeMap::new();
    let mut icons = BTreeMap::new();
    for line in data.lines().filter(|line| !line.starts_with('#')) {
        let fields: Vec<_> = line.split('\t').collect();
        let [_, _, icon, _, prefixes] = fields.as_slice() else {
            panic!("Invalid distro icon entry: {line}");
        };
        let next = icons.len();
        let icon_index = *icons.entry(*icon).or_insert(next);
        for prefix in prefixes.split(',') {
            assert!(
                !prefix.is_empty()
                    && prefix
                        .bytes()
                        .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit())
                    && aliases.insert(prefix, icon_index).is_none(),
                "Invalid or duplicate distro prefix: {prefix}"
            );
        }
    }

    let mut slots: Vec<_> = icons.into_iter().collect();
    slots.sort_unstable_by_key(|&(_, slot)| slot);
    let mut output = String::from("static ICONS: &[&Icon] = &[\n");
    for (icon, _) in slots {
        assert!(
            icon.bytes()
                .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
        );
        let path = format!("assets/distro/{icon}.rgba");
        println!("cargo:rerun-if-changed={path}");
        output.push_str(&format!(
            "    include_bytes!(concat!(env!(\"CARGO_MANIFEST_DIR\"), \"/{path}\")),\n"
        ));
    }
    output.push_str("];\nstatic PREFIXES: &[(&[u8], usize)] = &[\n");
    let mut aliases: Vec<_> = aliases.into_iter().collect();
    aliases.sort_by_key(|(prefix, _)| std::cmp::Reverse(prefix.len()));
    for (prefix, slot) in aliases {
        output.push_str(&format!("    (b\"{prefix}\", {slot}),\n"));
    }
    output.push_str("];\n");
    fs::write(
        PathBuf::from(env::var_os("OUT_DIR").expect("Cargo output directory"))
            .join("distro_icons.rs"),
        output,
    )
    .expect("write generated distro icon table");
}
