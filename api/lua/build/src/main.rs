use indexmap::{IndexMap, IndexSet};
use prost_types::{
    field_descriptor_proto::{Label, Type},
    DescriptorProto, EnumDescriptorProto, FieldDescriptorProto, ServiceDescriptorProto,
};

const CLIENT_CODE: &str = include_str!("../grpc_client.lua");

type EnumMap = IndexMap<String, EnumData>;
type MessageMap = IndexMap<String, MessageData>;

#[derive(Debug)]
struct MessageData {
    fields: Vec<Field>,
}

#[derive(Debug)]
struct Field {
    name: String,
    label: Option<Label>,
    r#type: FieldType,
}

#[derive(Debug)]
enum FieldType {
    Builtin(Type),
    Message(String),
    Enum(String),
}

fn parse_message_enums(enums: &mut EnumMap, prefix: &str, message: &DescriptorProto) {
    let prefix = format!("{prefix}.{}", message.name());
    for r#enum in message.enum_type.iter() {
        parse_enum(enums, &prefix, r#enum);
    }

    for msg in message.nested_type.iter() {
        let name = msg.name();
        parse_message_enums(enums, &format!("{prefix}.{name}"), msg);
    }
}

fn parse_enum(enums: &mut EnumMap, prefix: &str, enum_desc: &EnumDescriptorProto) {
    let name = enum_desc.name().to_string();

    let values = enum_desc.value.iter().map(|val| {
        let name = val.name().to_string();
        let number = val.number.unwrap();

        EnumValue { name, number }
    });

    enums.insert(
        format!("{prefix}.{name}"),
        EnumData {
            values: values.collect(),
        },
    );
}

fn parse_message(msgs: &mut MessageMap, prefix: &str, message: &DescriptorProto) {
    let name = format!("{prefix}.{}", message.name());

    let mut fields = Vec::new();

    for field in message.field.iter() {
        fields.push(parse_field(field));
    }

    let data = MessageData { fields };

    msgs.insert(name.clone(), data);

    for msg in message.nested_type.iter() {
        parse_message(msgs, &name, msg);
    }
}

fn parse_field(field: &FieldDescriptorProto) -> Field {
    Field {
        name: field.name().to_string(),
        label: field.label.is_some().then_some(field.label()),
        r#type: {
            if let Some(type_name) = field.type_name.as_ref() {
                if let Some(r#type) = field.r#type.is_some().then_some(field.r#type()) {
                    match r#type {
                        Type::Enum => FieldType::Enum(type_name.clone()),
                        Type::Message => FieldType::Message(type_name.clone()),
                        _ => panic!(),
                    }
                } else {
                    FieldType::Builtin(field.r#type())
                }
            } else {
                FieldType::Builtin(field.r#type())
            }
        },
    }
}

#[derive(Debug)]
struct EnumData {
    values: Vec<EnumValue>,
}

#[derive(Debug)]
struct EnumValue {
    name: String,
    number: i32,
}

fn generate_enum_definitions(enums: &EnumMap) -> String {
    let mut ret = String::new();

    for (name, data) in enums.iter() {
        let mut table = format!("---@enum {name}\nlocal {} = {{\n", name.replace('.', "_"));

        for val in data.values.iter() {
            table += &format!("    {} = {},\n", &val.name, val.number);
        }

        table += "}\n\n";

        ret += &table;
    }

    ret
}

fn generate_message_classes(msgs: &MessageMap) -> String {
    let mut ret = Vec::new();

    for (name, data) in msgs.iter() {
        if name == "google.protobuf.Empty" {
            ret.push("---@alias google.protobuf.Empty nil".to_string());
            ret.push(String::new());
            continue;
        }

        ret.push(format!("---@class {name}"));

        for field in data.fields.iter() {
            let r#type = match &field.r#type {
                FieldType::Builtin(builtin) => match builtin {
                    Type::Double | Type::Float => "number",
                    Type::Int32
                    | Type::Int64
                    | Type::Uint32
                    | Type::Uint64
                    | Type::Fixed64
                    | Type::Fixed32
                    | Type::Sfixed32
                    | Type::Sfixed64
                    | Type::Sint32
                    | Type::Sint64 => "integer",
                    Type::Bool => "boolean",
                    Type::String | Type::Bytes => "string",
                    Type::Group | Type::Message | Type::Enum => "any",
                }
                .to_string(),
                FieldType::Message(s) | FieldType::Enum(s) => s.trim_start_matches('.').to_string(),
            };

            let non_nil = if field
                .label
                .is_some_and(|label| matches!(label, Label::Required))
            {
                ""
            } else {
                "?"
            };

            let repeated = if field
                .label
                .is_some_and(|label| matches!(label, Label::Repeated))
            {
                "[]"
            } else {
                ""
            };

            ret.push(format!(
                "---@field {} {type}{repeated}{non_nil}",
                &field.name
            ));
        }

        ret.push(String::new());
    }

    ret.join("\n")
}

struct Visited {
    children: IndexMap<String, Visited>,
}

fn generate_message_tables(msgs: &MessageMap) -> String {
    let mut ret = Vec::new();

    let mut visited = IndexMap::<String, Visited>::new();

    for name in msgs.keys() {
        let segments = name.trim_start_matches('.').split('.').collect::<Vec<_>>();
        // let last = segments.last().unwrap().to_string();
        let mut current = &mut visited;

        let mut prev_segments = Vec::new();

        for segment in segments {
            current = &mut current
                .entry(segment.to_string())
                .or_insert_with(|| {
                    if prev_segments.is_empty() {
                        ret.push(format!("local {segment} = {{}}"));
                    } else {
                        ret.push(format!(
                            "{} = {{}}",
                            prev_segments
                                .iter()
                                .chain([&segment])
                                .copied()
                                .collect::<Vec<_>>()
                                .join(".")
                        ));
                    }
                    Visited {
                        children: IndexMap::new(),
                    }
                })
                .children;

            prev_segments.push(segment);
        }
    }

    ret.join("\n")
}

fn populate_table_enums(enums: &EnumMap) -> String {
    let mut ret = String::new();

    for name in enums.keys() {
        let name = name.trim_start_matches('.');
        let type_name = name.replace('.', "_");

        ret += &format!("{name} = {type_name}\n");
    }

    ret
}

fn populate_service_defs(prefix: &str, service: &ServiceDescriptorProto) -> String {
    let mut ret = Vec::new();

    let name = format!("{prefix}.{}", service.name());

    ret.push(format!("{name} = {{}}"));

    for method in service.method.iter() {
        let method_name = method.name();
        ret.push(format!("{name}.{method_name} = {{}}"));
        ret.push(format!("{name}.{method_name}.service = \"{name}\""));
        ret.push(format!("{name}.{method_name}.method = \"{method_name}\""));
        ret.push(format!(
            "{name}.{}.request = \"{}\"",
            method.name(),
            method.input_type()
        ));
        ret.push(format!(
            "{name}.{}.response = \"{}\"",
            method.name(),
            method.output_type()
        ));

        let client_method_name = format!("{name}.{method_name}").replace('.', "_");

        match (method.client_streaming(), method.server_streaming()) {
            // Bidirectional
            (true, true) => {
                ret.push(format!(
                    r#"
---Performs a bidirectional-streaming request.
---
---`callback` will be called with every streamed response.
---
---The raw client-to-server stream is returned to allow you to send encoded messages.
---
---@nodiscard
---
---@param callback fun(response: {ret_ty}, stream: grpc_client.h2.Stream)
---
---@return grpc_client.h2.Stream | nil
---@return string | nil An error string, if any
function Client:{client_method_name}(callback)
    return self:bidirectional_streaming_request({name}.{method_name}, callback)
end"#,
                    ret_ty = method.output_type().trim_start_matches('.'),
                ));
            }
            // Client-streaming
            (true, false) => {
                ret.push("-- Client-streaming unimplemented".to_string());
            }
            // Server-streaming
            (false, true) => {
                ret.push(format!(
                    r#"
---Performs a server-streaming request.
---
---`callback` will be called with every streamed response.
---
---@nodiscard
---
---@param data {data_ty}
---@param callback fun(response: {ret_ty})
---
---@return string | nil An error string, if any
function Client:{client_method_name}(data, callback)
    return self:server_streaming_request({name}.{method_name}, data, callback)
end"#,
                    data_ty = method.input_type().trim_start_matches('.'),
                    ret_ty = method.output_type().trim_start_matches('.'),
                ));
            }
            // Unary
            (false, false) => {
                ret.push(format!(
                    r#"
---Performs a unary request.
---
---@nodiscard
---
---@param data {data_ty}
---
---@return {ret_ty} | nil response
---@return string | nil error An error string, if any
function Client:{client_method_name}(data)
    return self:unary_request({name}.{method_name}, data)
end"#,
                    data_ty = method.input_type().trim_start_matches('.'),
                    ret_ty = method.output_type().trim_start_matches('.'),
                ));
            }
        }
    }

    ret.join("\n")
}

fn generate_returned_table(msgs: &MessageMap) -> String {
    let mut toplevel_packages = IndexSet::new();

    for name in msgs.keys() {
        let toplevel_package = name.trim_start_matches('.').split('.').next();
        if let Some(toplevel_package) = toplevel_package {
            toplevel_packages.insert(toplevel_package.to_string());
        }
    }

    let mut ret = String::from("return {\n");

    for pkg in toplevel_packages {
        ret += &format!("    {pkg} = {pkg},\n");
    }

    ret += "    grpc_client = grpc_client,\n";

    ret += "}\n";

    ret
}

fn main() {
    let Some(proto_dir) = std::env::args().nth(1) else {
        eprintln!("Usage: ./lua-build <proto-dir>");
        return;
    };

    let mut proto_paths = Vec::new();

    for entry in walkdir::WalkDir::new(&proto_dir).sort_by_file_name() {
        let entry = entry.unwrap();

        if entry.file_type().is_file() && entry.path().extension().is_some_and(|ext| ext == "proto")
        {
            proto_paths.push(entry.into_path());
        }
    }

    let file_descriptor_set = prost_build::Config::new()
        .load_fds(&proto_paths, &[proto_dir])
        .unwrap();

    let mut enums = EnumMap::new();
    let mut msgs = MessageMap::new();

    let mut services = Vec::new();

    for proto in file_descriptor_set.file.iter() {
        let package = proto.package().to_string();
        for r#enum in proto.enum_type.iter() {
            parse_enum(&mut enums, &package, r#enum);
        }

        for msg in proto.message_type.iter() {
            parse_message_enums(&mut enums, &package, msg);
            parse_message(&mut msgs, &package, msg);
        }

        for service in proto.service.iter() {
            services.push(populate_service_defs(&package, service));
        }
    }

    println!(
        "---@lcat nodoc\n\n{}",
        CLIENT_CODE.to_string()
            + "\n"
            + &generate_enum_definitions(&enums)
            + "\n"
            + &generate_message_classes(&msgs)
            + "\n"
            + &generate_message_tables(&msgs)
            + "\n"
            + &populate_table_enums(&enums)
            + "\n"
            + &services.join("\n")
            + "\n"
            + &generate_returned_table(&msgs)
    );
}
