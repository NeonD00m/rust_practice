pub mod c_cpp;
pub mod go;
pub mod js_ts;
pub mod lua;
pub mod php;
pub mod python;
pub mod rust;
pub mod web;

use crate::core::types::{ImportEdge, Language};
use std::path::Path;

pub fn extract_imports(
    content: &str,
    lang: Language,
    current_file: &Path,
    project_root: &Path,
) -> Vec<ImportEdge> {
    match lang {
        Language::C | Language::Cpp => {
            c_cpp::extract_c_cpp_imports(content, current_file, project_root)
        }
        Language::Python => python::extract_python_imports(content, current_file, project_root),
        Language::JavaScript | Language::TypeScript => {
            js_ts::extract_js_ts_imports(content, current_file, project_root)
        }
        Language::Lua | Language::Luau => {
            lua::extract_lua_imports(content, current_file, project_root)
        }
        Language::Rust => rust::extract_rust_imports(content, current_file, project_root),
        Language::Go => go::extract_go_imports(content, current_file, project_root),
        Language::PHP => php::extract_php_imports(content, current_file, project_root),
        Language::HTML | Language::CSS => {
            web::extract_web_imports(content, lang, current_file, project_root)
        }
        _ => Vec::new(),
    }
}
