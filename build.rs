use shaderc;
use std::path::{Path, PathBuf};

fn main() {
    println!("cargo:rerun-if-changed=shaders");

    for entry in std::fs::read_dir("shaders").expect("Shaders directory should exist") {
        let entry = entry.unwrap();
        let path = entry.path();

        if let Some(extension) = path.extension().and_then(|os_str| os_str.to_str()) {
            match extension.to_ascii_lowercase().as_str() {
                ext @ "vert" | ext @ "frag" => {
                    println!("cargo:rerun-if-changed={}", path.to_string_lossy());
                    let shader_kind = match ext {
                        "vert" => shaderc::ShaderKind::Vertex,
                        "frag" => shaderc::ShaderKind::Fragment,
                        _ => panic!("Unexpected shader type"),
                    };

                    compile_shader(path, shader_kind);
                },
                _ => {},
            }
        }
    }
}

fn compile_shader<P: AsRef<Path>>(path: P, shader_kind: shaderc::ShaderKind) {
    let path = path.as_ref();
    let mut output_path: PathBuf = path.to_path_buf();
    let extension = output_path.extension().unwrap().to_str().unwrap().to_string() + ".spv";
    output_path.set_extension(extension.to_string());

    let shader_source = std::fs::read_to_string(path).expect("Shader source should be available");
    let mut compiler = shaderc::Compiler::new().unwrap();

    let binary_result = compiler
        .compile_into_spirv(&shader_source, shader_kind, &path.to_string_lossy(), "main", None)
        .unwrap();

    std::fs::write(output_path, binary_result.as_binary_u8())
        .expect("Couldn't write SPIR-V shader file");
}
