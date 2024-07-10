use prost::bytes::Bytes;
use std::collections::HashMap;
use std::default;
use std::env;
use std::ffi::OsString;
use std::fs;
use std::io::{Error, ErrorKind, Result};
use std::path::PathBuf;
use std::process::Command;

use prost::Message;
use prost_types::{FileDescriptorProto, FileDescriptorSet};

mod ast;
mod code_generator;
mod extern_paths;
mod ident;
mod message_graph;
mod path;
mod protobuf;

use ast::Service;
use code_generator::CodeGenerator;
use extern_paths::ExternPaths;
use ident::to_snake;
use message_graph::MessageGraph;
use path::PathMap;

pub trait ServiceGenerator {
    fn generate(&self, service: Service, buf: &mut String);

    fn finalize(&mut self, _buf: &mut String) {}

    fn finalize_package(&mut self, _package: &str, _buf: &mut String) {}
}

#[derive(Clone, Copy)]
enum MapType {
    HashMap,
    BTreeMap,
}

impl Default for MapType {
    fn default() -> MapType {
        MapType::HashMap
    }
}

#[derive(Clone, Copy)]
enum BytesType {
    Bytes,
    Vec,
}

impl Default for BytesType {
    fn default() -> BytesType {
        BytesType::Vec
    }
}

pub struct Config {
    file_descriptor_set_path: Option<PathBuf>,
    service_generator: Option<Box<dyn ServiceGenerator>>,
    map_type: PathMap<MapType>,
    bytes_type: PathMap<BytesType>,
    type_attributes: PathMap<String>,
    message_attributes: PathMap<String>,
    enum_attributes: PathMap<String>,
    field_attributes: PathMap<String>,
    boxed: PathMap<()>,
    prost_types: bool,
    strip_enum_prefix: bool,
    out_dir: Option<PathBuf>,
    extern_paths: Vec<(String, String)>,
    default_package_filename: String,
    protoc_args: Vec<OsString>,
    disable_comments: PathMap<()>,
    skip_protoc_run: bool,
    include_file: Option<PathBuf>,
    prost_path: Option<String>,
    fmt: bool,
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

impl Config {
    pub fn new() -> Config {
        Config::default()
    }

    fn compile_fds(&mut self, file_descriptor_set: FileDescriptorSet) -> Result<()> {
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
            .map(|req| (req.0.clone(), req.0.to_file_name_or("mod.rs")))
            .collect::<HashMap<Module, String>>();

        let modules = self.generate(requests).unwrap();
        for (module, content) in &modules {
            let file_name = file_names.get(module).unwrap();
            let output_path = target.join(file_name);
            fs::write(output_path, content).unwrap();
        }

        Ok(())
    }

    pub fn generate(
        &mut self,
        requests: Vec<(Module, FileDescriptorProto)>,
    ) -> Result<HashMap<Module, String>> {
        let mut modules = HashMap::new();
        let mut packages = HashMap::new();

        let message_graph = MessageGraph::new(requests.iter().map(|x| &x.1))
            .map_err(|e| Error::new(ErrorKind::InvalidInput, e))?;
        let extern_paths = ExternPaths::new(&self.extern_paths, self.prost_types)
            .map_err(|e| Error::new(ErrorKind::InvalidInput, e))?;

        for (request_module, request_fd) in requests {
            if !request_fd.service.is_empty() {
                packages.insert(request_module.clone(), request_fd.package().to_string());
            }
            let buf = modules
                .entry(request_module.clone())
                .or_insert_with(String::new);
            CodeGenerator::generate(self, &message_graph, &extern_paths, request_fd, buf);
        }

        Ok(modules)
    }
    pub fn compile_protos(&mut self, protos: &[&str], includes: &[&str]) -> Result<()> {
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
                    Err(std::io::Error::new(
                        std::io::ErrorKind::Other,
                        "protoc failed",
                    ))
                }
            }
            Err(e) => Err(e),
        };
        dbg!(&result);
        let buf = fs::read(file_descriptor_set_path)?;

        let file_descriptor_set =
            FileDescriptorSet::decode(&*buf).map_err(|e| Error::new(ErrorKind::InvalidInput, e))?;

        self.compile_fds(file_descriptor_set)
    }
}

impl default::Default for Config {
    fn default() -> Config {
        Config {
            file_descriptor_set_path: None,
            service_generator: None,
            map_type: PathMap::default(),
            bytes_type: PathMap::default(),
            type_attributes: PathMap::default(),
            message_attributes: PathMap::default(),
            enum_attributes: PathMap::default(),
            field_attributes: PathMap::default(),
            boxed: PathMap::default(),
            prost_types: false,
            strip_enum_prefix: false,
            out_dir: None,
            extern_paths: Vec::new(),
            default_package_filename: "mod.rs".to_owned(),
            protoc_args: Vec::new(),
            disable_comments: PathMap::default(),
            skip_protoc_run: false,
            include_file: None,
            prost_path: None,
            fmt: false,
        }
    }
}

pub fn compile_protos(protos: &[&str], includes: &[&str]) -> Result<()> {
    Config::new().compile_protos(protos, includes)
}
