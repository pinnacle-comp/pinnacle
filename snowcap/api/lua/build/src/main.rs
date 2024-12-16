use std::collections::HashMap;

use indexmap::{IndexMap, IndexSet};
use prost::Message as _;
use prost_types::{
    field_descriptor_proto::{Label, Type},
    DescriptorProto, EnumDescriptorProto, FieldDescriptorProto, ServiceDescriptorProto,
};

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

    let mut fields: HashMap<Option<i32>, Vec<Field>> = HashMap::new();

    for field in message.field.iter() {
        fields
            // .entry(field.oneof_index)
            .entry(None)
            .or_default()
            .push(parse_field(field));
    }

    let data = MessageData {
        fields: fields.remove(&None).unwrap_or_default(),
    };

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
    let mut ret = String::new();

    for (name, data) in msgs.iter() {
        let mut class = format!("---@class {name}\n");

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

            class += &format!("---@field {} {type}{repeated}{non_nil}\n", &field.name);
        }

        class += "\n";

        ret += &class;
    }

    ret
}

struct Visited {
    children: HashMap<String, Visited>,
}

fn generate_message_tables(msgs: &MessageMap) -> String {
    let mut ret = String::new();

    let mut visited = HashMap::<String, Visited>::new();

    for name in msgs.keys() {
        let segments = name.trim_start_matches('.').split('.');
        let mut current = &mut visited;

        let mut prev_segments = Vec::new();

        for segment in segments {
            current = &mut current
                .entry(segment.to_string())
                .or_insert_with(|| {
                    if prev_segments.is_empty() {
                        ret += &format!("local {segment} = {{}}\n")
                    } else {
                        ret += &format!(
                            "{} = {{}}\n",
                            prev_segments
                                .iter()
                                .chain([&segment])
                                .copied()
                                .collect::<Vec<_>>()
                                .join(".")
                        );
                    }
                    Visited {
                        children: HashMap::new(),
                    }
                })
                .children;

            prev_segments.push(segment);
        }
    }

    ret
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
    let mut ret = String::new();

    let name = format!("{prefix}.{}", service.name());

    ret += &format!("{name} = {{}}\n");

    for method in service.method.iter() {
        ret += &format!("{name}.{} = {{}}\n", method.name());
        ret += &format!("{name}.{}.service = \"{name}\"\n", method.name());
        ret += &format!("{name}.{n}.method = \"{n}\"\n", n = method.name());
        ret += &format!(
            "{name}.{}.request = \"{}\"\n",
            method.name(),
            method.input_type()
        );
        ret += &format!(
            "{name}.{}.response = \"{}\"\n",
            method.name(),
            method.output_type()
        );
    }

    ret
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

    ret += "}\n";

    ret
}

fn main() {
    let file_descriptor_set_bytes = include_bytes!(concat!(env!("OUT_DIR"), "/lua-build.bin"));
    let file_descriptor_set =
        prost_types::FileDescriptorSet::decode(&file_descriptor_set_bytes[..]).unwrap();

    let mut enums = EnumMap::new();
    let mut msgs = MessageMap::new();

    let mut services = String::new();

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
            services += &populate_service_defs(&package, service);
        }
    }

    println!(
        "{}",
        generate_enum_definitions(&enums) + "\n"
        + &generate_message_classes(&msgs) + "\n"
        + &generate_message_tables(&msgs) + "\n"
        // + &populate_message_tables(&msgs)
        + &populate_table_enums(&enums) + "\n" + &services + "\n" + &generate_returned_table(&msgs)
    );
}
