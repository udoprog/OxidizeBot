use failure::format_err;
use hal::pso::{EntryPoint, Specialization};
use std::{fs, io::Read, path::Path};

const ENTRY_NAME: &str = "main";

pub struct Shader<'a, D, B>
where
    D: hal::Device<B>,
    B: hal::Backend,
{
    device: &'a D,
    module: B::ShaderModule,
}

impl<'a, D, B> Shader<'a, D, B>
where
    D: hal::Device<B>,
    B: hal::Backend,
{
    /// Open the given path as a shader module.
    pub fn open(
        device: &'a D,
        path: impl AsRef<Path>,
        shader_type: glsl_to_spirv::ShaderType,
    ) -> Result<Shader<'a, D, B>, failure::Error> {
        let glsl = fs::read_to_string(path)?;

        let spirv: Vec<u8> = glsl_to_spirv::compile(&glsl, shader_type)
            .map_err(|e| format_err!("{}", e))?
            .bytes()
            .collect::<Result<Vec<_>, _>>()?;

        let module = unsafe { device.create_shader_module(&spirv) }?;

        Ok(Shader { device, module })
    }

    /// Get the entry point to the shader module.
    pub fn entry_point(&self) -> EntryPoint<'_, B> {
        EntryPoint {
            entry: ENTRY_NAME,
            module: &self.module,
            specialization: Specialization::default(),
        }
    }
}

impl<'a, D, B> Drop for Shader<'a, D, B>
where
    D: hal::Device<B>,
    B: hal::Backend,
{
    fn drop(&mut self) {
        use std::mem;

        unsafe {
            self.device
                .destroy_shader_module(mem::replace(&mut self.module, mem::zeroed()));
        }
    }
}
