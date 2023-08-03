use std::process::Command;
use std::io::Result;
use std::fs;
use std::env;
use std::path::PathBuf;
use prost::Message;

include!("./protobuf.rs");
include!("./message_graph.rs");
include!("./code_generator.rs");

pub fn to_snake(s: &str) -> String {
    s.to_string()
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Module {
    components: Vec<String>,
}

impl Module {
    pub fn from_protobuf_package_name(name: &str) -> Self {
        Self {
            components: name
                .split('.')
                .filter(|s| !s.is_empty())
                .map(to_snake)
                .collect(),
        }
    }

    fn to_file_name_or(&self, default: &str) -> String {
        if self.components.is_empty() {
            default.to_owned()
        } else {
            self.components.join("/") + ".rs"
        }
    }
}

fn generate(requests: Vec<(Module, FileDescriptorProto)>) -> Result<HashMap<Module, String>> {
    let mut modules = HashMap::new();
    let mut packages = HashMap::new();

    let message_graph = MessageGraph::new(requests.iter().map(|x| &x.1));

    for (request_module, request_fd) in requests {
        if !request_fd.service.is_empty() {
            packages.insert(request_module.clone(), request_fd.package().to_string());
        }
        let buf = modules
            .entry(request_module.clone())
            .or_insert_with(String::new);
        CodeGenerator::generate(config, &message_graph, &extern_paths, request_fd, buf);
    }

    Ok(modules)
}

fn compile_file_descriptor_set(file_descriptor_set: FileDescriptorSet) {
    let target: PathBuf = env::var("OUT_DIR").unwrap().into();

    let requests = file_descriptor_set
        .file
        .into_iter()
        .map(|descriptor| {
            (
                Module::from_protobuf_package_name(descriptor.package()),
                descriptor,
            )
        })
        .collect::<Vec<_>>();

    let file_names = requests
        .iter()
        .map(|req| {
            (
                req.0.clone(),
                req.0.to_file_name_or("mod.rs"),
                )
        })
        .collect::<HashMap<Module, String>>();

    let modules = generate(requests);
    for (module, content) in &modules {
        let file_name = file_names.get(module).unwrap();
        let output_path = target.join(file_name);
        fs::write(output_path, content).unwrap();
    }
}

pub fn compile_protos(protos: &[&str], includes: &[&str]) -> Result<()> {
    let file_descriptor_set_path = "/tmp/prost-build-descriptor-set.pb";
    eprintln!("Compiling protos");
    let mut cmd = Command::new("protoc");
    cmd.arg("--include_imports")
        .arg("--include_source_info")
        .arg("-o")
        .arg(&file_descriptor_set_path);

    for include in includes {
        // TODO make this a path and check if it exists
        cmd.arg("-I").arg(include);
    }

    for proto in protos {
        cmd.arg(proto);
    }

    println!("Running {:?}", cmd);

    let output = cmd.output();

    let result = match output {
        Ok(output) => {
            if output.status.success() {
                Ok(())
            } else {
                Err(std::io::Error::new(std::io::ErrorKind::Other, "protoc failed"))
            }
        }
        Err(e) => Err(e),
    };
    dbg!(&result);
    let buf = fs::read(file_descriptor_set_path)?;

    let file_descriptor_set = FileDescriptorSet::decode(&*buf)?;

    compile_file_descriptor_set(file_descriptor_set);

    Ok(())
}
