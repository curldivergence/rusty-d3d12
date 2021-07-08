import re
import sys


def caps_to_camel_case(string):
    result = string.split("_")
    return ''.join(x.title() for x in result)

camel_ptrn = re.compile(r'(?<!^)(?=[A-Z])')
def camel_case_to_snake(string):
    return camel_ptrn.sub('_', string).lower()

def parse_flags():
    lines = ""
    print("Paste flags enum definition from bindings.rs:")
    while True:
        ln = input()
        if ln:
            lines += ln
        else:
            break

    lines = lines            \
        .replace("\r", "")   \
        .replace("\n", "")   \
        .replace(";", ";\n") \
        .strip()             \
        .split("\n")

    # print(lines)

    # print(enum_type_ptrn.match(lines[-1]))
    enum_type_ptrn = re.compile(r"^pub\s+type\s+(.*)\s+=\s+::std::os::raw::c_([a-z]+);\s*$")

    raw_enum_name = None
    enum_name = None
    enum_type = None
    for ln in lines:
        m = enum_type_ptrn.match(ln)
        if not m:
            continue
        raw_enum_name, enum_type = m.groups()[0], m.groups()[1]
        if enum_type == "int":
            enum_type = "i32"
        elif enum_type == "uint":
            enum_type = "u32"
        break

    if not enum_type:
        raise TypeError

    enum_name = caps_to_camel_case(raw_enum_name).replace("D3D12", "")
    print(f"Detected enum name: raw '{raw_enum_name}', formatted '{enum_name}'"
        f", type: '{enum_type}'")

    print("Enter prefix that should be stripped from enum variants:")
    strip_prefix = input()
    # strip_prefix = "D3D12_DESCRIPTOR_RANGE_FLAGS_D3D12_DESCRIPTOR_RANGE_FLAG_"
    enum_variant_ptrn = re.compile(f"^pub\s+const\s+{strip_prefix}([a-zA-Z0-9_]+)\s*:\s+{raw_enum_name}\s+=\s+.*$")
    print(f"Created pattern for matching enum variants: {enum_variant_ptrn}")

    raw_enum_variants = []
    enum_variants = []

    for ln in lines:
        m = enum_variant_ptrn.match(ln)
        if not m:
            continue
        raw_variant_name = m.groups()[0]
        raw_enum_variants.append(raw_variant_name)
        variant_name = caps_to_camel_case(raw_variant_name)
        enum_variants.append(variant_name)
        print(f"Found variant name: raw '{raw_variant_name}', formatted '{variant_name}'")

    # print(enum_variants)

    bitflags_source = f"""
bitflags! {{
    pub struct {enum_name}: {enum_type} {{
"""
    for r, f in zip(raw_enum_variants, enum_variants):
        bitflags_source += " " * 8 + f"const {f} = {strip_prefix}{r};\n"
    bitflags_source += " " * 4 + "}\n" + "}\n"

    print(bitflags_source)


def parse_enum():
    lines = ""
    print("Paste enum definition from bindings.rs:")
    while True:
        ln = input()
        if ln:
            lines += ln
        else:
            break

    lines = lines            \
        .replace("\r", "")   \
        .replace("\n", "")   \
        .replace(";", ";\n") \
        .strip()             \
        .split("\n")

    # print(lines)

    # print(enum_type_ptrn.match(lines[-1]))
    enum_type_ptrn = re.compile(r"^pub\s+type\s+(.*)\s+=\s+::std::os::raw::c_([a-z]+);\s*$")

    raw_enum_name = None
    enum_name = None
    enum_type = None
    for ln in lines:
        m = enum_type_ptrn.match(ln)
        if not m:
            continue
        raw_enum_name, enum_type = m.groups()[0], m.groups()[1]
        if enum_type == "int":
            enum_type = "i32"
        elif enum_type == "uint":
            enum_type = "u32"
        break

    if not enum_type:
        raise TypeError

    enum_name = caps_to_camel_case(raw_enum_name).replace("D3D12", "")
    print(f"Detected enum name: raw '{raw_enum_name}', formatted '{enum_name}'"
        f", type: '{enum_type}'")

    print("Enter prefix that should be stripped from enum variants:")
    strip_prefix = input()
    # strip_prefix = "D3D12_DESCRIPTOR_RANGE_FLAGS_D3D12_DESCRIPTOR_RANGE_FLAG_"
    enum_variant_ptrn = re.compile(f"^pub\s+const\s+{strip_prefix}([a-zA-Z0-9_]+)\s*:\s*{raw_enum_name}\s+=\s+.*$")
    print(f"Created pattern for matching enum variants: {enum_variant_ptrn}")

    raw_enum_variants = []
    enum_variants = []

    for ln in lines:
        m = enum_variant_ptrn.match(ln)
        if not m:
            continue
        raw_variant_name = m.groups()[0]
        raw_enum_variants.append(raw_variant_name)
        variant_name = caps_to_camel_case(raw_variant_name)
        enum_variants.append(variant_name)
        # print(f"Found variant name: raw '{raw_variant_name}', formatted '{variant_name}'")

    # print(enum_variants)

    enum_definition_begin = f"""
#[derive(Copy, Clone)]
#[repr(i32)]
pub enum {enum_name} {{"""
    print(enum_definition_begin)

    enum_body = ""
    for r, f in zip(raw_enum_variants, enum_variants):
        enum_body += " " * 4 + f"{f} = {strip_prefix}{r},\n"
    print(enum_body[:-1]) # strip extra empty line

    enum_definition_end = "}"
    print(enum_definition_end)


def parse_struct():
    lines = ""
    print("Paste struct definition from bindings.rs:")
    while True:
        ln = input()
        if ln:
            lines += ln
        else:
            break

#     lines = """pub struct D3D12_VERSIONED_DEVICE_REMOVED_EXTENDED_DATA {
#     pub Version: D3D12_DRED_VERSION,
#     pub __bindgen_anon_1:
#         D3D12_VERSIONED_DEVICE_REMOVED_EXTENDED_DATA__bindgen_ty_1,
# }
# """

#     lines = """pub struct D3D12_DESCRIPTOR_RANGE {
#     pub RangeType: D3D12_DESCRIPTOR_RANGE_TYPE,
#     pub NumDescriptors: UINT,
#     pub BaseShaderRegister: UINT,
#     pub RegisterSpace: UINT,
#     pub OffsetInDescriptorsFromTableStart: UINT,
# }
# """

    lines = lines            \
        .replace("\r", "")   \
        .replace("\n", "")   \
        .replace("{", "{\n") \
        .replace(",", ",\n") \
        .strip()             \
        .split("\n")

    # print(lines)

    struct_name_ptrn = re.compile(r"^\s*pub\s+struct\s+([a-zA-Z0-9_]+)\s+{\s*$")
    struct_member_ptrn = re.compile(r"^\s*pub\s+([a-zA-Z0-9_]+)\s*:\s+([a-zA-Z0-9_ \*\[\]:;]+)\s*,\s*$")

    raw_struct_name = None
    struct_name = None
    raw_members = []

    for ln in lines:
        m = struct_name_ptrn.match(ln)
        if m:
            raw_struct_name = m.groups()[0]
            continue
        m = struct_member_ptrn.match(ln)
        if m:
            raw_members.append((m.groups()[0], m.groups()[1]))

    if not raw_struct_name:
        raise TypeError

    # print(raw_struct_name, raw_members)

    struct_name = caps_to_camel_case(raw_struct_name).replace("D3D12", "")
    struct_definition = f"""#[derive(Default)]
#[repr(transparent)]
pub struct {struct_name}(pub {raw_struct_name});
"""

    print(struct_definition)

    struct_impl_begin = f"impl {struct_name} {{"
    print(struct_impl_begin)
    all_methods_source = ""
    for raw_name, ty in raw_members:
        member_name = camel_case_to_snake(raw_name)
        setter_source = f"""    pub fn set_{member_name}(mut self, {member_name}: {ty}) -> Self {{
        self.0.{raw_name} = {member_name};
        self
    }}

""" \
            .replace("UINT64", "u64") \
            .replace("UINT", "u32") \
            .replace("FLOAT", "f32")

        all_methods_source += setter_source
        member_name = camel_case_to_snake(raw_name)

        getter_source = f"""    pub fn get_{member_name}(&self) -> {ty} {{
        self.0.{raw_name}
    }}

""" \
            .replace("UINT64", "u64") \
            .replace("UINT", "u32") \
            .replace("FLOAT", "f32")

        all_methods_source += getter_source

    print(all_methods_source[:-2]) # strip newline after last method
    # print(repr(all_setters_source[-5:]))

    struct_impl_end = "}\n"
    print(struct_impl_end)



if sys.argv[1] == "flags":
    parse_flags()
elif sys.argv[1] == "enum":
    parse_enum()
elif sys.argv[1] == "struct":
    parse_struct()
else:
    print("Unknown command")