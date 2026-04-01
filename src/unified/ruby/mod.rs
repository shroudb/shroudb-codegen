//! Ruby unified SDK generator.

mod agents;
mod client;
mod engine;
mod errors;
mod package;
mod readme;
mod transport;
mod types;

use super::UnifiedGenerator;
use super::ir::UnifiedIR;
use crate::generator::GeneratedFile;

pub struct RubyUnifiedGenerator;

impl UnifiedGenerator for RubyUnifiedGenerator {
    fn language(&self) -> &'static str {
        "Ruby"
    }

    fn generate(&self, ir: &UnifiedIR) -> Vec<GeneratedFile> {
        let mut files = Vec::new();
        files.extend(package::generate(ir));
        files.extend(transport::generate(ir));
        files.extend(errors::generate(ir));
        files.extend(types::generate(ir));
        for engine in &ir.engines {
            files.extend(engine::generate(ir, engine));
        }
        files.extend(client::generate(ir));
        files.push(readme::generate(ir));
        files.push(agents::generate(ir));
        files
    }
}
