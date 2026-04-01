//! go.mod generation for the Go SDK.

use crate::generator::GeneratedFile;
use crate::unified::ir::UnifiedIR;

pub(super) fn generate(ir: &UnifiedIR) -> Vec<GeneratedFile> {
    vec![gen_go_mod(ir)]
}

fn gen_go_mod(ir: &UnifiedIR) -> GeneratedFile {
    let module = &ir.packages.go_module;

    GeneratedFile {
        path: "go.mod".into(),
        content: format!(
            "module {module}\n\
             \n\
             go 1.21\n"
        ),
    }
}
