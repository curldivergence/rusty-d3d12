import re
import sys
import os
from enum import Enum

def caps_to_camel_case(string):
    result = string.split("_")
    return ''.join(x.title() for x in result)

def trim_struct_version(struct_name):
    return re.sub(r'[0-9]+$', '', struct_name)

def trim_prefix(struct_name):
    trimmed_name = re.sub(r'^D3D12', '', struct_name)
    trimmed_name = re.sub(r'^D3D', '', trimmed_name)
    trimmed_name = re.sub(r'^DXGI', '', trimmed_name)
    return trimmed_name

def rustify_type_name(raw_type_name):
    rusty_type_name = trim_prefix(raw_type_name)
    rusty_type_name = caps_to_camel_case(rusty_type_name)
    rusty_type_name = trim_struct_version(rusty_type_name)

    return rusty_type_name

class TypeCategory(Enum):
    BuiltIn = 0,
    BitFlags = 1,
    Enum = 2,
    Struct = 3

# needed so that not only can we replace type name in setters/getters,
# but also can generate appropriate conversion code
KNOWN_TYPES = {}
struct_decl_ptrn = re.compile(r'pub struct (.*)\(pub\(crate\) .*\);')
enum_decl_ptrn = re.compile(r'pub enum (.*) {')
flags_decl_ptrn = re.compile(r'pub struct (.*):')

def read_known_types():
    with open(os.path.join(os.getcwd(), "src", "struct_wrappers.rs")) as struct_wrappers_file:
        source_code = struct_wrappers_file.readlines()

        for line in source_code:
            m = struct_decl_ptrn.findall(line)
            if m:
                KNOWN_TYPES[m[0]] = TypeCategory.Struct
                continue

    with open(os.path.join(os.getcwd(), "src", "enum_wrappers.rs")) as enum_wrappers_file:
        source_code = enum_wrappers_file.readlines()

        for line in source_code:
            m = enum_decl_ptrn.findall(line)
            if m:
                KNOWN_TYPES[m[0]] = TypeCategory.Enum
                continue

            m = flags_decl_ptrn.findall(line)
            if m:
                KNOWN_TYPES[m[0]] = TypeCategory.BitFlags
                continue

    # print(KNOWN_TYPES)

def replace_known_types(ty):
    if ty == "UINT64":
        return "u64"
    elif ty == "SIZE_T":
        return "u64"
    elif ty == "UINT16":
        return "u16"
    elif ty == "UINT8":
        return "u8"
    elif ty == "BYTE":
        return "u8"
    elif ty == "UINT":
        return "u32"
    elif ty == "INT":
        return "i32"
    elif ty == "FLOAT":
        return "f32"
    elif ty == "LONG":
        return "i32"
    elif ty == "BOOL":
        return "bool"

    if "D3D12" in ty or "DXGI" in ty:
        return rustify_type_name(ty)

camel_ptrn = re.compile(r'(?<!^)(?=[A-Z])')
def camel_case_to_snake(string):
    return camel_ptrn.sub('_', string).lower()

def parse_flags():
    lines = ""
    print("Paste flags enum definition from d3d12.rs:")
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

    enum_name = rustify_type_name(raw_enum_name)
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
    print("Paste enum definition from d3d12.rs:")
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

    enum_name = rustify_type_name(raw_enum_name)
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
        if variant_name[0].isdigit():
            variant_name = enum_name[0] + variant_name
        enum_variants.append(variant_name)
        # print(f"Found variant name: raw '{raw_variant_name}', formatted '{variant_name}'")

    # print(enum_variants)

    enum_definition_begin = f"""
#[derive(Debug, Copy, Clone)]
#[repr(i32)]
pub enum {enum_name} {{"""
    print(enum_definition_begin)

    enum_body = ""
    for r, f in zip(raw_enum_variants, enum_variants):
        enum_body += " " * 4 + f"{f} = {strip_prefix}{r},\n"
    print(enum_body[:-1]) # strip extra empty line

    enum_definition_end = "}"
    print(enum_definition_end)

def generate_setter_common(raw_name, rusty_name, ty):
    return f"""    pub fn set_{rusty_name}(&mut self, {rusty_name}: {ty}) -> &mut Self {{
        self.0.{raw_name} = {rusty_name};
        self
    }}

"""

def generate_getter_common(raw_name, rusty_name, ty):
    return f"""    pub fn {rusty_name}(&self) -> {ty} {{
        self.0.{raw_name}
    }}

"""

def generate_setter_for_bool(raw_name, rusty_name, ty):
    return f"""    pub fn set_{rusty_name}(&mut self, {rusty_name}: {ty}) -> &mut Self {{
        self.0.{raw_name} = {rusty_name} as i32;
        self
    }}

"""

def generate_getter_for_bool(raw_name, rusty_name, ty):
    return f"""    pub fn {rusty_name}(&self) -> {ty} {{
        self.0.{raw_name} != 0
    }}

"""

def generate_setter_for_struct(raw_name, rusty_name, ty):
    return f"""    pub fn set_{rusty_name}(&mut self, {rusty_name}: {ty}) -> &mut Self {{
        self.0.{raw_name} = {rusty_name}.0;
        self
    }}

"""

def generate_getter_for_struct(raw_name, rusty_name, ty):
    return f"""    pub fn {rusty_name}(&self) -> {ty} {{
        {ty}(self.0.{raw_name})
    }}

"""

def generate_setter_for_enum(raw_name, rusty_name, ty):
    return f"""    pub fn set_{rusty_name}(&mut self, {rusty_name}: {ty}) -> &mut Self {{
        self.0.{raw_name} = {rusty_name} as i32;
        self
    }}

"""

def generate_getter_for_enum(raw_name, rusty_name, ty):
    return f"""    pub fn {rusty_name}(&self) -> {ty} {{
        unsafe {{ std::mem::transmute(self.0.{raw_name}) }}
    }}

"""

def generate_setter_for_flags(raw_name, rusty_name, ty):
    return f"""    pub fn set_{rusty_name}(&mut self, {rusty_name}: {ty}) -> &mut Self {{
        self.0.{raw_name} = {rusty_name}.bits();
        self
    }}

"""

def generate_getter_for_flags(raw_name, rusty_name, ty):
    return f"""    pub fn {rusty_name}(&self) -> {ty} {{
        unsafe {{ {ty}::from_bits_unchecked(self.0.{raw_name}) }}
    }}

"""

def parse_struct():
    lines = ""
    print("Paste struct definition from d3d12.rs:")
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

    struct_name_ptrn = re.compile(r"^\s*pub\s+(struct|union)\s+([a-zA-Z0-9_]+)\s+{\s*$")
    struct_member_ptrn = re.compile(r"^\s*pub\s+([a-zA-Z0-9_]+)\s*:\s+([a-zA-Z0-9_ \*\[\]:;]+)\s*,\s*$")

    raw_struct_name = None
    struct_name = None
    raw_members = []

    for ln in lines:
        m = struct_name_ptrn.match(ln)
        if m:
            raw_struct_name = m.groups()[1]
            continue
        m = struct_member_ptrn.match(ln)
        if m:
            raw_members.append((m.groups()[0], m.groups()[1]))

    if not raw_struct_name:
        raise TypeError

    # print(raw_struct_name, raw_members)

    struct_name = rustify_type_name(raw_struct_name)
    struct_definition = f"""/// Wrapper around {raw_struct_name} structure
#[derive(Default, Debug, Hash, PartialOrd, Ord, PartialEq, Eq, Clone)]
#[repr(transparent)]
pub struct {struct_name}(pub(crate) {raw_struct_name});
"""

    print(struct_definition)

    struct_impl_begin = f"impl {struct_name} {{"
    print(struct_impl_begin)
    all_methods_source = ""
    for raw_name, ty in raw_members:
        ty = replace_known_types(ty)
        member_name = camel_case_to_snake(raw_name)
        if ty == "bool":
            set_method_source = generate_setter_for_bool(raw_name, member_name, ty)
        elif ty in KNOWN_TYPES:
            if KNOWN_TYPES[ty] == TypeCategory.Struct:
                set_method_source = generate_setter_for_struct(raw_name, member_name, ty)
            elif KNOWN_TYPES[ty] == TypeCategory.Enum:
                set_method_source = generate_setter_for_enum(raw_name, member_name, ty)
            elif KNOWN_TYPES[ty] == TypeCategory.BitFlags:
                set_method_source = generate_setter_for_flags(raw_name, member_name, ty)
            else:
                print("Trying to generate setter for an unknown type")
                exit(1)
        else:
            set_method_source = generate_setter_common(raw_name, member_name, ty)

        all_methods_source += set_method_source

        with_method_source = f"""    pub fn with_{member_name}(mut self, {member_name}: {ty}) -> Self {{
        self.set_{member_name}({member_name});
        self
    }}

"""
        all_methods_source += with_method_source

        member_name = camel_case_to_snake(raw_name)

        if ty == "bool":
            getter_source = generate_getter_for_bool(raw_name, member_name, ty)
        elif ty in KNOWN_TYPES:
            if KNOWN_TYPES[ty] == TypeCategory.Struct:
                getter_source = generate_getter_for_struct(raw_name, member_name, ty)
            elif KNOWN_TYPES[ty] == TypeCategory.Enum:
                getter_source = generate_getter_for_enum(raw_name, member_name, ty)
            elif KNOWN_TYPES[ty] == TypeCategory.BitFlags:
                getter_source = generate_getter_for_flags(raw_name, member_name, ty)
            else:
                print("Trying to generate getter for an unknown type")
                exit(1)
        else:
            getter_source = generate_getter_common(raw_name, member_name, ty)
        all_methods_source += getter_source

    print(all_methods_source[:-2]) # strip newline after last method
    # print(repr(all_setters_source[-5:]))

    struct_impl_end = "}\n"
    print(struct_impl_end)


read_known_types()

if sys.argv[1] == "flags":
    parse_flags()
elif sys.argv[1] == "enum":
    parse_enum()
elif sys.argv[1] == "struct":
    parse_struct()
else:
    print("Unknown command")