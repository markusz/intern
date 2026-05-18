use intern_core::{
    model::EMU_PER_PX,
    reader::read_presentation,
    rules::{Limits, RuleContext, all_rules},
};

fn main() {
    let path = std::env::args().nth(1).unwrap_or_else(|| {
        eprintln!("usage: lint <file.pptx>");
        std::process::exit(1);
    });

    let pres = match read_presentation(&path) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("error: {e}");
            std::process::exit(1);
        }
    };

    let ctx = RuleContext {
        threshold: 2 * EMU_PER_PX, // 2px
        slide_width: pres.slide_width,
        slide_height: pres.slide_height,
    };
    let rules = all_rules(&Limits::default());

    let violations: Vec<_> = rules
        .iter()
        .flat_map(|r| r.check(&pres.slides, &ctx))
        .collect();

    if violations.is_empty() {
        println!("No violations found.");
        return;
    }

    for v in &violations {
        let slide = v.slide.map(|n| format!("slide {n}")).unwrap_or_default();
        let element = v.element.map(|id| format!("#{id}")).unwrap_or_default();
        println!("[{}] {slide} {element} - {}", v.rule_id, v.message);
    }

    println!("\n{} violation(s)", violations.len());
    std::process::exit(1);
}
