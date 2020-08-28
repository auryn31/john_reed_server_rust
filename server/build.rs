extern crate prost_build;

fn main() {
    prost_build::compile_protos(&["src/model/proto_model.proto"], &["src/model/"]).unwrap();
}
