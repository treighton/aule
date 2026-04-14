pub mod adapter_def;
pub mod generate;
pub mod paths;
pub mod registry;
pub mod script;
pub mod target;

pub use adapter_def::{AdapterDef, AdapterDefError, AdapterSource, ConfigAdapter, ScriptAdapter};
pub use adapter_def::{AdapterPaths, AdapterFrontmatter, CommandConfig};
pub use adapter_def::{parse_adapter_def, parse_adapter_def_from_path};
pub use generate::{generate, generate_v2, generate_any, GenerateOptions, GeneratedFile, GenerateError};
pub use paths::expand_home;
pub use registry::{AdapterRegistry, AdapterEntry};
pub use script::{ScriptInput, ScriptContent, ScriptOptions, ScriptOutput, ScriptExecError};
// Keep RuntimeTarget for backward compatibility
pub use target::RuntimeTarget;
