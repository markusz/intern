use intern_core::{
    model::EMU_PER_PX,
    reader::read_presentation,
    rules::{Limits, all_rules},
};

fn main() {
    let path = std::env::args().nth(1).unwrap_or_else(|| {
        eprintln!("usage: lint <file.pptx>");
        std::process::exit(1);
    });

    let slides = match read_presentation(&path) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("error: {e}");
            std::process::exit(1);
        }
    };

    let threshold = 2 * EMU_PER_PX; // 2px
    let rules = all_rules(&Limits::default());

    let violations: Vec<_> = rules
        .iter()
        .flat_map(|r| r.check(&slides, threshold))
        .collect();

    if violations.is_empty() {
        println!("No violations found.");
        return;
    }

    for v in &violations {
        let slide = v.slide.map(|n| format!("slide {n}")).unwrap_or_default();
        let element = v.element.as_deref().unwrap_or("");
        println!("[{}] {slide} {element} — {}", v.rule_id, v.message);
    }

    println!("\n{} violation(s)", violations.len());
    std::process::exit(1);
}
