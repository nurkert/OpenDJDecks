//! Writes the hardware status of every bundled controller mapping into the
//! controllers guide (docs/guide/{en,de}/controllers.md), between the
//! `hardware-status` markers, and the controller list (bundled mappings and
//! the guided-setup classes) between the `controller-list` markers. Without
//! `--write` it prints the English tables.
//!
//! cargo run -p od-controller --example hardware_status -- --write

use od_controller::Mapping;
use od_controller::guided::Template;
use od_controller::hardware::{controller_list_markdown, replace_list, replace_section, status_markdown};
use std::path::PathBuf;
use std::sync::Arc;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let dir = od_controller::bundled_mappings_dir().join("controllers");
    let mut loaded = Mapping::load_dir(&dir);
    loaded.sort_by(|a, b| a.0.cmp(&b.0));
    let mappings: Vec<Arc<Mapping>> = loaded
        .into_iter()
        .map(|(p, m)| m.map(Arc::new).map_err(|e| format!("{}: {e}", p.display())))
        .collect::<Result<_, _>>()?;
    if !std::env::args().any(|a| a == "--write") {
        print!("{}", controller_list_markdown(&mappings, &Template::all(), false));
        print!("{}", status_markdown(&mappings, false));
        return Ok(());
    }
    let guide = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../docs/guide");
    for (lang, de) in [("en", false), ("de", true)] {
        let path = guide.join(lang).join("controllers.md");
        let doc = std::fs::read_to_string(&path)?;
        let new = replace_section(&doc, &status_markdown(&mappings, de))
            .ok_or_else(|| format!("{}: hardware-status markers missing", path.display()))?;
        let new = replace_list(&new, &controller_list_markdown(&mappings, &Template::all(), de))
            .ok_or_else(|| format!("{}: controller-list markers missing", path.display()))?;
        std::fs::write(&path, new)?;
        println!("wrote {}", path.display());
    }
    Ok(())
}
